use crate::intern::Name;
use crate::source::Span;
use crate::types::{SymId, TypeId};

macro_rules! id_type {
    ($($name:ident),*) => {$(
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
        pub struct $name(pub u32);
        impl $name {
            #[inline]
            pub fn idx(self) -> usize { self.0 as usize }
        }
    )*};
}
id_type!(ExprId, PatId, TyExprId, DefId, StrId);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListRef {
    pub start: u32,
    pub len: u32,
}

impl ListRef {
    pub const EMPTY: ListRef = ListRef { start: 0, len: 0 };
    #[inline]
    pub fn range(self) -> std::ops::Range<usize> {
        self.start as usize..(self.start + self.len) as usize
    }
    #[inline]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Expr {
    IntLit(i64),
    LongLit(i64),
    DoubleLit(f64),
    /// A decimal or exponent literal without a suffix (`1.5`, `1e3`, `-2.0`): its digits as
    /// written, the sign of a `-` the parser folded in included and the separators left out. It
    /// is an untyped number, as dotty's `Parsers.literal` makes it, whose type and value the
    /// expected type gives (`Typer.typedNumber`): a `Float` read from the digits themselves where
    /// one is expected, a `Double` otherwise.
    DecimalLit(StrId),
    FloatLit(f32),
    BoolLit(bool),
    CharLit(u32),
    StringLit(StrId),
    UnitLit,
    Ident(Name),
    /// A reference resolved by the loader: a definition of a library body's own file.
    SymRef(SymId),
    This,
    /// `C.this` of an enclosing class or object `C` in a library body, as the resolved type of `C`.
    ThisOf(TyExprId),
    /// `C.this`, or the `self =>` alias of an enclosing template: the name of the class or of
    /// the alias.
    QualThis(Name),
    /// `classOf[T]` in a library body.
    ClassOf(TyExprId),
    /// A construct of a library body the converter has no reading for; typing it reports the
    /// reason, so that only a body the program reaches fails.
    Unsupported(StrId),
    /// A body a product's pickle withholds (`ELIDED`): typing it fails the
    /// build with the message, which names the definition, the module and the part that writes it.
    Withheld(StrId),
    /// `super`, or `super[T]` with the name of the parent; always the qualifier of a selection.
    Super(Name),
    Select(ExprId, Name),
    Apply(ExprId, ListRef),
    UsingApply(ExprId, ListRef),
    NamedArg(Name, ExprId),
    TypeApply(ExprId, ListRef),
    Infix(ExprId, Name, ExprId),
    Prefix(Name, ExprId),
    /// Params live in `Ast::lambda_params`.
    Lambda(ListRef, ExprId),
    /// `[T] => (x: A) => body`, a polymorphic function literal: the type parameter names live
    /// in `Ast::name_lists`, the rest is a `Lambda`.
    PolyLambda(ListRef, ExprId),
    If(ExprId, ExprId, Option<ExprId>),
    /// Cases live in `Ast::cases`.
    Match(ExprId, ListRef),
    /// `inline if`: reduced on a constant condition when the enclosing inline method expands.
    InlineIf(ExprId, ExprId, Option<ExprId>),
    /// `inline x match`: reduced on the static type or constant value of the scrutinee.
    InlineMatch(ExprId, ListRef),
    /// Statements live in `Ast::stmts`.
    Block(ListRef),
    While(ExprId, ExprId),
    /// Enumerators live in `Ast::enumerators`.
    For(ListRef, ExprId, bool),
    Assign(ExprId, ExprId),
    Tuple(ListRef),
    /// `(a = x, b = y)`: the names live in `Ast::name_lists`, the values in `Ast::expr_lists`.
    NamedTuple(ListRef, ListRef),
    /// `((a, b))`: a tuple in its own parentheses is one infix argument, `a op (b, c)` has two.
    Parens(ExprId),
    Typed(ExprId, TyExprId),
    /// `e: @unchecked`, which keeps a match on `e` from being checked for exhaustivity.
    Unchecked(ExprId),
    /// `new C[T](args)`, needed when the companion object defines its own `apply`.
    New(TyExprId, ListRef),
    /// `new T with U { ... }`: a class definition made up by the parser, whose parents carry the
    /// types and constructor arguments written after `new`.
    NewAnon(DefId),
    /// Parts live in `Ast::str_lists`, arguments in `Ast::expr_lists`.
    Interp(Name, ListRef, ListRef),
    /// The right-hand side of a given that a `derives` clause stands for: `TC.derived`, where the
    /// given implements `TC[T]`.
    Derived,
    Return(Option<ExprId>),
    NullLit,
    Throw(ExprId),
    /// An index into `Ast::tries`.
    Try(u32),
    /// `'{ e }` or `'x`: the code of `e` as an `Expr`.
    Quote(ExprId),
    /// `'[T]`: the type `T` as a `Type`.
    QuoteType(TyExprId),
    /// `${ e }` or `$x`: inside a quote the code that `e` evaluates to, at the top level of an
    /// inline method the macro that expands the call.
    Splice(ExprId),
    /// `${ p }` or `$x` inside a quote pattern: the code standing here is matched against `p`.
    SplicePat(PatId),
    Error,
}

#[derive(Clone, Copy, Debug)]
pub struct TryExpr {
    pub body: ExprId,
    /// The cases of `catch { case ... }`; EMPTY when the handler is an expression or absent.
    pub cases: ListRef,
    /// `catch handler` with an expression, a `PartialFunction[Throwable, T]`.
    pub handler: Option<ExprId>,
    pub finalizer: Option<ExprId>,
}

/// The entry of `Ast::lambda_bounds` for a parameter without a bound.
pub const NO_BOUND: TyExprId = TyExprId(u32::MAX);

/// A lambda's type parameter's lower and upper bound as written (dotty's `TypeBoundsTree`).
pub type LambdaBounds = (Option<TyExprId>, Option<TyExprId>);

#[derive(Clone, Copy, Debug)]
pub struct LambdaParam {
    pub name: Name,
    pub span: Span,
    pub ty: Option<TyExprId>,
    /// `implicit x => ...`: the parameter is a given inside the body.
    pub implicit: bool,
    /// `x ?=> ...`: the lambda is a context function, its parameters givens inside the body.
    pub contextual: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Stmt {
    Expr(ExprId),
    Def(DefId),
    /// An index into `Ast::import_stmts`.
    Import(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct CaseClause {
    pub pat: PatId,
    pub guard: Option<ExprId>,
    pub body: ExprId,
}

#[derive(Clone, Copy, Debug)]
pub enum Enumerator {
    Gen(PatId, ExprId),
    /// `case pat <- e`: elements the pattern does not match are filtered out.
    CaseGen(PatId, ExprId),
    Guard(ExprId),
    Val(PatId, ExprId),
}

#[derive(Clone, Copy, Debug)]
pub enum Pat {
    Wildcard,
    Bind(Name, Option<PatId>),
    Typed(PatId, TyExprId),
    Lit(ExprId),
    StableId(ExprId),
    Ctor(ExprId, ListRef),
    Tuple(ListRef),
    /// `name = pattern` inside a tuple or constructor pattern.
    NamedField(Name, PatId),
    Alt(ListRef),
    /// `xs*` or `_*` ending a sequence pattern; the inner pattern is a binder or a wildcard.
    Rest(PatId),
    /// `case '{ ... }`: the quoted code as a pattern, its splices `$x` binding the code they
    /// stand for and its lowercase type names binding types.
    Quote(ExprId),
    /// `case '[T]`: a type pattern over a `Type`.
    QuoteType(TyExprId),
    /// `case s"a-$x"`: an interpolated string as a pattern, with the interpolator's name, the
    /// literal parts (`Ast::str_lists`) and the patterns of its holes, matched through
    /// `StringContext(parts).<name>.unapplySeq`.
    Interp(Name, ListRef, ListRef),
    Error,
}

#[derive(Clone, Copy, Debug)]
pub enum TyExpr {
    Name(Name),
    Select(TyExprId, Name),
    Apply(TyExprId, ListRef),
    /// `(A, B) => C`; parameter names written with the types are in `Ast::fun_param_names`.
    Fun(ListRef, TyExprId),
    /// `A ?=> B`
    CtxFun(ListRef, TyExprId),
    Tuple(ListRef),
    /// `(a: A, b: B)`: the names live in `Ast::name_lists`, the types in `Ast::ty_lists`.
    NamedTuple(ListRef, ListRef),
    Union(TyExprId, TyExprId),
    Inter(TyExprId, TyExprId),
    ByName(TyExprId),
    Repeated(TyExprId),
    Wildcard,
    /// `? >: L <: H`, a missing bound a `Resolved` `Nothing` or `Any`.
    BoundedWildcard(TyExprId, TyExprId),
    /// Parameter names live in `Ast::name_lists`.
    Lambda(ListRef, TyExprId),
    /// `[T] => (A, B) => C`, a polymorphic function type: the type parameter names live in
    /// `Ast::name_lists`, their bounds in `Ast::lambda_bounds`, the function type is a `Fun`.
    PolyFun(ListRef, TyExprId),
    /// `path.type`; the path is a chain of `Name` and `Select`.
    Singleton(TyExprId),
    /// `T#A`
    Project(TyExprId, Name),
    /// `T { type A = X; def m: Int }`; the members are definitions in `Ast::def_lists`.
    Refined(TyExprId, ListRef),
    /// A literal type; the expression is the literal.
    Lit(ExprId),
    /// A type the loader mapped already, in a library body.
    Resolved(TypeId),
    /// A lowercase type argument of a pattern type (`case l: List[t]`): a type variable the
    /// case binds. An `inline match` binds it to what it matched, an ordinary match reads it
    /// as a wildcard.
    TypeVar(Name),
    /// `T @unchecked`: a type test against it is not checked for being decidable.
    Unchecked(TyExprId),
    /// `T @uncheckedVariance`: left out of the variance check.
    UncheckedVariance(TyExprId),
    /// `S match { case P => T ... }`; the list holds `MatchCase` nodes.
    Match(TyExprId, ListRef),
    /// One case of a match type; the lowercase names of the pattern are `TypeVar`s it binds.
    MatchCase(TyExprId, TyExprId),
    Error,
}

pub type Mods = u32;

pub mod mods {
    use super::Mods;
    pub const PRIVATE: Mods = 1 << 0;
    pub const PROTECTED: Mods = 1 << 1;
    pub const SEALED: Mods = 1 << 2;
    pub const ABSTRACT: Mods = 1 << 3;
    pub const FINAL: Mods = 1 << 4;
    pub const CASE: Mods = 1 << 5;
    pub const LAZY: Mods = 1 << 6;
    pub const OVERRIDE: Mods = 1 << 7;
    pub const OPAQUE: Mods = 1 << 8;
    pub const INLINE: Mods = 1 << 9;
    pub const OPEN: Mods = 1 << 10;
    pub const INFIX: Mods = 1 << 11;
    pub const MUTABLE: Mods = 1 << 12;
    pub const FIELD: Mods = 1 << 13;
    /// `class C private (...)`.
    pub const PRIVATE_CTOR: Mods = 1 << 14;
    /// A given whose name the parser made up from its type.
    pub const ANONYMOUS: Mods = 1 << 15;
    /// A Scala 2 `implicit` definition: a val, def, object or class that the given search
    /// treats as a given, and a def with one plain parameter as a conversion. The TASTy loader
    /// sets it on the definitions it reads with the `Implicit` flag.
    pub const IMPLICIT: Mods = 1 << 16;
    /// A `given` read from TASTy, where it stands on a val, def or object.
    pub const GIVEN: Mods = 1 << 17;
    pub const ENUM: Mods = 1 << 18;
    pub const EXTENSION: Mods = 1 << 19;
    /// `class C protected (...)`.
    pub const PROTECTED_CTOR: Mods = 1 << 20;
    /// `transparent inline def`: the call takes the type of the expanded body.
    pub const TRANSPARENT: Mods = 1 << 21;
    /// A jar's `private[p]` or `protected[p]` member, whose scope `Loaded::access_within` holds.
    pub const QUALIFIED: Mods = 1 << 22;
    /// A definition whose header the parser could not complete: a syntax error after its name,
    /// or a token after it that neither continues it nor ends the statement. What parsed of it
    /// is kept, and the typer reads what is missing as unknown.
    pub const INCOMPLETE: Mods = 1 << 23;
    /// A Java annotation interface read from its class file.
    pub const JAVA_ANNOTATION: Mods = 1 << 24;
    /// The setter `x_=` of an abstract `var x`: the member an assignment through the var calls
    /// and an implementing class defines. The namer makes it, without a definition, or the
    /// loader reads it from a pickle.
    pub const SETTER: Mods = 1 << 25;
    /// A deferred given: `given x: T = deferred` in a trait, the marker resolving to
    /// `scala.compiletime.deferred` (scalac's `Deferred | HasDefault`, set as its signature
    /// completes), or one read from a pickle with `HASDEFAULT` and no body. Abstract, and
    /// implemented by every class that is the first to extend its trait, by a search where the
    /// class is defined (`Worker::implement_deferred_givens`).
    pub const DEFERRED: Mods = 1 << 27;
}

#[derive(Clone, Debug)]
pub struct TypeParam {
    pub name: Name,
    pub span: Span,
    pub variance: i8,
    /// How many type parameters a higher-kinded parameter `F[_, _]` takes; 0 for a type.
    pub arity: u8,
    /// The variances a higher-kinded parameter declares for its own parameters (`F[+_]`);
    /// empty where they are all invariant.
    pub hk_variances: Vec<i8>,
    pub upper: Option<TyExprId>,
    pub lower: Option<TyExprId>,
    pub context_bounds: Vec<TyExprId>,
    /// The name of each context bound's evidence (`[A: Show as s]`), EMPTY when left to the
    /// compiler; parallel to `context_bounds`.
    pub evidence_names: Vec<Name>,
    pub annots: Vec<Annot>,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: Name,
    pub span: Span,
    pub ty: TyExprId,
    pub default: Option<ExprId>,
    pub mods: Mods,
    /// Annotations of a constructor parameter, in `Ast::param_annots`.
    pub annots: ListRef,
}

#[derive(Clone, Debug, Default)]
pub struct ParamClause {
    pub params: Vec<Param>,
    pub is_using: bool,
    /// A Scala 2 `(implicit ...)` clause: a using clause whose arguments may also be passed
    /// without `using`.
    pub is_implicit: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Annot {
    pub name: Name,
    /// String literal arguments; they live in `Ast::str_lists`.
    pub args: ListRef,
    /// The annotation read as `new path[targs](args)(more)`, which is how a mirror
    /// instantiates it: a `New`, applied to the lists after the first.
    pub instance: ExprId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClassKind {
    Class,
    Trait,
    Object,
    Enum,
    EnumCase,
}

#[derive(Clone, Debug)]
pub struct Parent {
    pub ty: TyExprId,
    /// Constructor argument lists, each with whether it is a `using` list.
    pub args: Vec<(ListRef, bool)>,
}

#[derive(Debug)]
pub struct FunDef {
    pub tparams: Vec<TypeParam>,
    pub clauses: Vec<ParamClause>,
    pub ret: Option<TyExprId>,
    pub body: Option<ExprId>,
    /// For extension methods: the leading type params and clauses (receiver included) that
    /// come from the enclosing `extension`.
    pub ext_tparams: u8,
    pub ext_clauses: u8,
    pub is_extension: bool,
    /// Numbers the `extension` clauses of a file from 1, so that the methods of one are told
    /// from those of another.
    pub ext_group: u32,
}

#[derive(Debug)]
pub struct ClassDef {
    pub kind: ClassKind,
    pub tparams: Vec<TypeParam>,
    pub clauses: Vec<ParamClause>,
    pub parents: Vec<Parent>,
    pub body: Vec<Stmt>,
    /// `export` clauses of the body, one entry per selector, in `Ast::exports`.
    pub exports: ListRef,
    /// The type of `self: T =>` at the start of the body.
    pub self_type: Option<TyExprId>,
    /// The name of `self =>` at the start of the body, or `EMPTY`.
    pub self_alias: Name,
}

#[derive(Debug)]
pub struct GivenDef {
    pub tparams: Vec<TypeParam>,
    pub clauses: Vec<ParamClause>,
    pub ty: TyExprId,
    pub alias: Option<ExprId>,
    pub body: Vec<Stmt>,
    /// `self =>` at the start of a `given ... with` body, as a class's.
    pub self_alias: Name,
}

#[derive(Debug)]
pub enum DefKind {
    Val { pat: Option<PatId>, ty: Option<TyExprId>, rhs: Option<ExprId> },
    Fun(Box<FunDef>),
    Class(Box<ClassDef>),
    /// `type T = X`, or without `rhs` the abstract member `type T >: L <: U`; an opaque alias
    /// has both `rhs` and `upper`.
    TypeAlias { tparams: Vec<TypeParam>, rhs: Option<TyExprId>, lower: Option<TyExprId>, upper: Option<TyExprId> },
    Given(Box<GivenDef>),
}

#[derive(Debug)]
pub struct Def {
    pub name: Name,
    pub span: Span,
    pub mods: Mods,
    pub annots: Vec<Annot>,
    pub kind: DefKind,
}

#[derive(Clone, Debug)]
pub enum ImportSel {
    Wildcard,
    Given,
    Name(Name, Option<Name>),
}

#[derive(Clone, Debug)]
pub struct Import {
    pub path: Vec<Name>,
    pub sel: ImportSel,
    /// From the head of the clause's path through this selector: what the selectors of one
    /// clause share the start of, and what the index and the unused-import marks key on.
    pub span: Span,
    /// The selector alone, where the unused-import warning stands: the plain name, the whole
    /// rename (`Shadow as Sh`, `a => b`, `a as _`), the `*` or `_`, the `given` with its type.
    pub selector_span: Span,
    /// The type of a `given T` selector, whose names use the imports they resolve through
    /// (`typer::unused`); the givens it brings are not filtered by it.
    pub bound: Option<TyExprId>,
}

pub struct Ast {
    pub exprs: Vec<Expr>,
    pub expr_spans: Vec<Span>,
    pub pats: Vec<Pat>,
    pub pat_spans: Vec<Span>,
    pub tys: Vec<TyExpr>,
    pub ty_spans: Vec<Span>,
    pub defs: Vec<Def>,
    pub strings: Vec<String>,
    pub expr_lists: Vec<ExprId>,
    pub pat_lists: Vec<PatId>,
    pub ty_lists: Vec<TyExprId>,
    pub def_lists: Vec<DefId>,
    pub str_lists: Vec<StrId>,
    /// Where the source writes the entries of `str_lists` an interpolation's parts are (the
    /// others' and those beyond the last interpolation's left out or empty): the part as
    /// written, which the string an `s` interpolation's escapes were processed in does not keep.
    pub str_list_spans: Vec<Span>,
    pub name_lists: Vec<Name>,
    /// The bounds of the parameters of a type lambda (`[x <: Node] =>> ...`, the `x` of an
    /// `E[+x <: Node]`) or a polymorphic function type (`[A >: String] => ..`), a list in
    /// `ty_lists` of each parameter's lower and upper bound, `NO_BOUND` for none.
    pub lambda_bounds: Vec<(TyExprId, ListRef)>,
    /// The same of a polymorphic function literal's parameters (`[A <: Int] => (a: A) => ..`).
    pub poly_lambda_bounds: Vec<(ExprId, ListRef)>,
    /// The parameter names of a function type that writes them (`(c: Ctx) => c.T`), a list in
    /// `name_lists` parallel to its parameters.
    pub fun_param_names: Vec<(TyExprId, ListRef)>,
    pub param_annots: Vec<Annot>,
    pub lambda_params: Vec<LambdaParam>,
    pub stmts: Vec<Stmt>,
    pub cases: Vec<CaseClause>,
    pub enumerators: Vec<Enumerator>,
    pub tries: Vec<TryExpr>,
    pub package: Vec<Name>,
    /// Whether the file is the block a `package object` was read as: its top-level
    /// definitions are the object's members, which scalac names through `package$`.
    pub package_object: bool,
    /// Where each `package` clause ends in `package`, so that `package a` followed by
    /// `package b.c` gives the clauses `a` and `a.b.c`.
    pub package_clauses: Vec<u32>,
    pub imports: Vec<Import>,
    /// Imports inside blocks and bodies, one entry per selector.
    pub local_imports: Vec<Import>,
    /// The range of each import statement in `local_imports`.
    pub import_stmts: Vec<ListRef>,
    pub exports: Vec<Import>,
    /// Package-level `export` clauses.
    pub top_exports: Vec<Import>,
    /// `import language.strictEquality` was written in the file.
    pub strict_equality: bool,
    /// The file's top-level imports of `scala.language`, which the parser takes out of
    /// `imports`: they name the object all the same.
    pub language_imports: Vec<Import>,
    /// The initialisers written `_` (`var x: T = _`), which only a field may have.
    pub default_inits: Vec<ExprId>,
    /// The names of one `val a, b = e` after the first, each with the name before it: every name
    /// has its own copy of `e` (`parser/defs.rs`), typed in this order whichever is asked for
    /// first (`typer/check.rs`).
    pub val_copies: crate::intern::FxMap<DefId, DefId>,
    /// `import scala.language.future`: the migration warnings of the next version are errors.
    pub source_future: bool,
    pub top_level: Vec<DefId>,
    /// `private[pkg]` and `protected[pkg]`: where the name of the definition or parameter
    /// starts, and `pkg`.
    pub access_scopes: Vec<(u32, Name)>,
    /// Whether the file quotes or splices anywhere.
    pub has_quotes: bool,
    /// A library body's `new p.C(..)` of a class nested in a class, with the prefix `p` that
    /// is the enclosing instance of the new one.
    pub new_outers: crate::intern::FxMap<ExprId, ExprId>,
    /// The binders `case given T` and `given T <- ..` make, named as scalac names an anonymous
    /// given (`given_T`): each is a given where it is in scope.
    pub given_binds: std::collections::HashSet<PatId, crate::intern::FxBuild>,
    /// The `new C()` written with an empty first argument list before further lists: the `()`
    /// is an argument list, never the place of a using clause.
    pub new_empty_first: Vec<ExprId>,
    /// A library body's type argument lists that scalac inferred, by their start in
    /// `ty_lists`, ascending: where they do not check, the typer infers the arguments again.
    pub inferred_type_lists: Vec<u32>,
    /// A library body's `new C[T](args)` with inferred type arguments whose using clause the
    /// AST leaves to the typer, with the call passing the body's own givens: what is typed where
    /// the type arguments are inferred again.
    pub inferred_alternatives: Vec<(ExprId, ExprId)>,
    /// A library body's declared types of lambda parameters and local vals that scalac
    /// inferred, ascending: where they do not admit what the context gives, that is taken.
    pub inferred_types: Vec<u32>,
    /// Per definition the range of the whole of it, from its annotations or modifiers to its
    /// last token; `NO_RANGE` where the parser made the definition up.
    pub def_ranges: Vec<Span>,
    /// With `Syntax::index`: the span of the name of each `Select`, `Infix` and `Prefix`
    /// expression, by expression in ascending order, which the expression's own span covers
    /// with its qualifier or operands.
    pub name_spans: Vec<(ExprId, Span)>,
    /// With `Syntax::index`: the span of the name of each `TyExpr::Select`, likewise.
    pub ty_name_spans: Vec<(TyExprId, Span)>,
    /// With `Syntax::index`: per import or export selector (by its `Import::span`) the spans of
    /// its path's names, then of the selector's name and of its rename when it has them.
    pub import_names: Vec<(Span, Vec<Span>)>,
    /// With `Syntax::index`: the whole range of each top-level `import` statement of this tree,
    /// from `import` to its last token (a brace included), and of each `package` clause of it (a
    /// block's own `package p:`), in source order: where completion's auto-import inserts.
    pub import_ranges: Vec<Span>,
    pub package_ranges: Vec<Span>,
    /// A library body's references as its TASTy states them; none in source.
    pub reader: Option<Box<ReaderTables>>,
    /// The annotations of ascriptions and types written inline (`e: @switch`, `T @nowarn`), in
    /// source order: no meaning is read of them but `@unchecked`'s and `@uncheckedVariance`'s,
    /// while their names use the imports they resolve through (`typer::unused`).
    pub inline_annots: Vec<Annot>,
    /// The tokens the parser skipped after a syntax error, each run with the construct whose
    /// recovery skipped it, in source order.
    pub recoveries: Vec<Recovery>,
    /// The argument lists the parser's recovery cut (tokens skipped, or its `)` missing), each
    /// by its first argument, or by its application or instantiation when it holds none: what
    /// the list holds stands, how many arguments it would have is unknown.
    pub cut_args: Vec<ExprId>,
    /// The cases, by their index in `cases`, ascending, whose pattern or guard reported a syntax
    /// error: a match's coverage over them is unknown.
    pub broken_cases: Vec<u32>,
}

/// What the converter of a library body keeps beside the AST it makes: the declaration each
/// selection names and the trace of each expansion, by the node made of it.
#[derive(Default)]
pub struct ReaderTables {
    pub decls: crate::intern::FxMap<ExprId, DeclRef>,
    pub traces: crate::intern::FxMap<ExprId, InlinedTrace>,
    /// Under `TEQ_READER_DUMP` alone: the owner of each declaration as the pickle spells it,
    /// which the listing shows beside the class it maps to.
    pub pickled_owners: crate::intern::FxMap<ExprId, String>,
    /// The std adaptations the conversion made (A2 to A9), by the node it made
    /// in the call's place: what the call lost for the std's member to take it.
    pub adaptations: Vec<(ExprId, &'static str)>,
    /// The bounds a typed pattern's `BIND`s state beyond `>: Nothing <: Any`, by the pattern:
    /// each variable's name, its lower and its upper bound.
    pub binder_bounds: crate::intern::FxMap<PatId, Vec<(Name, Option<TyExprId>, Option<TyExprId>)>>,
    /// The tree each node stands for.
    pub places: NodePlaces,
    /// A product's expansions as its producer recorded them, for the typer to record as the
    /// whole program's typing of the calls does.
    pub replay: Replay,
    /// The classes a product's SAM conversions made, by the converted lambda's body: the name
    /// its producer gave the class and the place of the lambda in its source (kind 2 at the
    /// `$anonfun`).
    pub sam_classes: crate::intern::FxMap<ExprId, (String, u32, u32)>,
}

/// What a product's `INLINED`s took from their call sites, by the nodes made of them: the
/// expansions with the methods they expand, the leaves, and the type tests (an expression's or
/// a pattern's) of the calls' type arguments.
#[derive(Default)]
pub struct Replay {
    pub expansions: crate::intern::FxMap<ExprId, crate::types::SymId>,
    pub leaves: crate::intern::FxMap<ExprId, ()>,
    pub test_exprs: crate::intern::FxMap<ExprId, ()>,
    pub test_pats: crate::intern::FxMap<PatId, ()>,
}

impl Replay {
    pub fn is_empty(&self) -> bool {
        self.expansions.is_empty()
    }
}

/// Per node of a converted body, the address in its TASTy file (the loader's `file`) of the
/// tree the node stands for: its own for a node made of a tree, the tree's whose conversion made
/// it for a node the converter adds.
#[derive(Default)]
pub struct NodePlaces {
    pub file: u32,
    pub exprs: Vec<u32>,
    pub pats: Vec<u32>,
    /// The tree a node stands for whole where `exprs` gives a part of it, the one a diagnostic
    /// names: an infix expression's application, its place the operator's selection.
    pub wholes: Vec<(u32, u32)>,
}

impl NodePlaces {
    /// The address of the tree the node `e` stands for whole.
    pub fn whole(&self, e: ExprId) -> Option<u32> {
        self.whole_of_part(e).or_else(|| self.exprs.get(e.idx()).copied())
    }

    /// The tree the node `e` stands for whole where its place is a part of it.
    pub fn whole_of_part(&self, e: ExprId) -> Option<u32> {
        let i = self.wholes.binary_search_by_key(&e.idx(), |&(i, _)| i as usize).ok()?;
        Some(self.wholes[i].1)
    }
}

/// A `SELECTin`'s declaration: the signed name (the name as declared, its target name and its
/// signature) in the name table of the TASTy file the loader numbers `file`, and the address in
/// that file's trees of the type of the class that declares it. The typer resolves it when it
/// types the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeclRef {
    pub file: u32,
    pub name: u32,
    pub owner_at: u32,
}

/// An `INLINED`'s trace: the address of its call in the body's TASTy file, which names the
/// top-level class the expansion came from after scalac's `PostTyper` (none for a tree inlined
/// from the caller's scope), and how many of the block's statements are its bindings. The
/// class is looked up only where the listing names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlinedTrace {
    pub call_at: Option<u32>,
    pub bindings: u32,
}

/// The construct whose recovery skipped tokens after a syntax error.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecoverySite {
    /// A token no statement of the top level or of a package block starts with.
    TopLevel,
    /// The tokens of a broken definition header before its `=` or its body on the same line.
    Header,
    /// A token no statement of a template body starts with.
    TemplateBody,
    /// A token no method of an extension starts with.
    Extension,
    /// A token no statement of a given's `with` body starts with.
    GivenBody,
    /// A token no statement of a block starts with.
    Block,
    /// A token between the cases of an indented match.
    Cases,
    /// A token no enumerator of an indented `for` starts with.
    ForEnumerators,
    /// A token no member of a refinement starts with.
    Refinement,
    /// A token between the cases of an indented match type.
    TypeCases,
    /// The tokens of a parenthesised, bracketed or braced list before its next comma or its
    /// closer, after a token that neither continues an element nor separates one.
    List,
    /// An indentation the top level does not expect.
    Indentation,
    /// A `:` at the end of a line with no indented body after it.
    ColonBody,
}

impl RecoverySite {
    pub fn label(self) -> &'static str {
        match self {
            RecoverySite::TopLevel => "top-level",
            RecoverySite::Header => "header",
            RecoverySite::TemplateBody => "template-body",
            RecoverySite::Extension => "extension",
            RecoverySite::GivenBody => "given-body",
            RecoverySite::Block => "block",
            RecoverySite::Cases => "cases",
            RecoverySite::ForEnumerators => "for-enumerators",
            RecoverySite::Refinement => "refinement",
            RecoverySite::TypeCases => "type-cases",
            RecoverySite::List => "list",
            RecoverySite::Indentation => "indentation",
            RecoverySite::ColonBody => "colon-body",
        }
    }
}

/// Tokens skipped by a recovery: from the first skipped token's start to the last one's end.
#[derive(Clone, Copy, Debug)]
pub struct Recovery {
    pub site: RecoverySite,
    pub span: Span,
}

/// The range of a definition the parser made up.
pub const NO_RANGE: Span = Span { start: u32::MAX, end: 0 };

impl Ast {
    pub fn new(size_hint: usize) -> Ast {
        let n = size_hint / 8 + 16;
        Ast {
            exprs: Vec::with_capacity(n),
            expr_spans: Vec::with_capacity(n),
            pats: Vec::new(),
            pat_spans: Vec::new(),
            tys: Vec::with_capacity(n / 4),
            ty_spans: Vec::with_capacity(n / 4),
            defs: Vec::new(),
            strings: Vec::new(),
            expr_lists: Vec::with_capacity(n / 2),
            pat_lists: Vec::new(),
            ty_lists: Vec::new(),
            def_lists: Vec::new(),
            str_lists: Vec::new(),
            str_list_spans: Vec::new(),
            name_lists: Vec::new(),
            lambda_bounds: Vec::new(),
            poly_lambda_bounds: Vec::new(),
            fun_param_names: Vec::new(),
            param_annots: Vec::new(),
            lambda_params: Vec::new(),
            stmts: Vec::new(),
            cases: Vec::new(),
            enumerators: Vec::new(),
            tries: Vec::new(),
            package: Vec::new(),
            package_object: false,
            package_clauses: Vec::new(),
            imports: Vec::new(),
            local_imports: Vec::new(),
            import_stmts: Vec::new(),
            exports: Vec::new(),
            top_exports: Vec::new(),
            strict_equality: false,
            language_imports: Vec::new(),
            default_inits: Vec::new(),
            val_copies: crate::intern::FxMap::default(),
            source_future: false,
            top_level: Vec::new(),
            access_scopes: Vec::new(),
            has_quotes: false,
            new_outers: crate::intern::FxMap::default(),
            given_binds: Default::default(),
            new_empty_first: Vec::new(),
            inferred_type_lists: Vec::new(),
            inferred_alternatives: Vec::new(),
            inferred_types: Vec::new(),
            def_ranges: Vec::new(),
            name_spans: Vec::new(),
            ty_name_spans: Vec::new(),
            import_names: Vec::new(),
            import_ranges: Vec::new(),
            package_ranges: Vec::new(),
            reader: None,
            inline_annots: Vec::new(),
            recoveries: Vec::new(),
            cut_args: Vec::new(),
            broken_cases: Vec::new(),
        }
    }

    /// The range of the whole definition, or its name's span where the parser has none.
    pub fn def_range(&self, id: DefId) -> Span {
        match self.def_ranges.get(id.idx()) {
            Some(&r) if r != NO_RANGE => r,
            _ => self.defs[id.idx()].span,
        }
    }

    /// The span of the name of a selection or an operator, `None` for another expression or
    /// without the index.
    /// The spans the language server's index reads (`Syntax::index`) at their length: a std
    /// file's tree, parsed once and kept by every build of a session.
    pub fn shrink_index_spans(&mut self) {
        self.name_spans.shrink_to_fit();
        self.ty_name_spans.shrink_to_fit();
        self.import_names.shrink_to_fit();
        self.import_ranges.shrink_to_fit();
        self.package_ranges.shrink_to_fit();
    }

    pub fn name_span(&self, id: ExprId) -> Option<Span> {
        let i = self.name_spans.binary_search_by_key(&id.0, |(e, _)| e.0).ok()?;
        Some(self.name_spans[i].1)
    }

    pub fn ty_name_span(&self, id: TyExprId) -> Option<Span> {
        let i = self.ty_name_spans.binary_search_by_key(&id.0, |(t, _)| t.0).ok()?;
        Some(self.ty_name_spans[i].1)
    }

    #[inline]
    pub fn expr(&self, id: ExprId) -> Expr {
        self.exprs[id.idx()]
    }
    #[inline]
    pub fn pat(&self, id: PatId) -> Pat {
        self.pats[id.idx()]
    }
    #[inline]
    pub fn ty(&self, id: TyExprId) -> TyExpr {
        self.tys[id.idx()]
    }
    /// The parameter names a function type writes, `c` of `(c: Ctx) => c.T`.
    pub fn fun_param_names(&self, id: TyExprId) -> Option<ListRef> {
        self.fun_param_names.iter().find(|&&(f, _)| f == id).map(|&(_, n)| n)
    }
    #[inline]
    pub fn def(&self, id: DefId) -> &Def {
        &self.defs[id.idx()]
    }
    #[inline]
    pub fn str(&self, id: StrId) -> &str {
        &self.strings[id.idx()]
    }
    #[inline]
    pub fn expr_span(&self, id: ExprId) -> Span {
        self.expr_spans[id.idx()]
    }
    #[inline]
    pub fn expr_list(&self, l: ListRef) -> &[ExprId] {
        &self.expr_lists[l.range()]
    }
    #[inline]
    pub fn pat_list(&self, l: ListRef) -> &[PatId] {
        &self.pat_lists[l.range()]
    }
    #[inline]
    pub fn ty_list(&self, l: ListRef) -> &[TyExprId] {
        &self.ty_lists[l.range()]
    }
    #[inline]
    pub fn def_list(&self, l: ListRef) -> &[DefId] {
        &self.def_lists[l.range()]
    }
    #[inline]
    pub fn stmt_list(&self, l: ListRef) -> &[Stmt] {
        &self.stmts[l.range()]
    }
    #[inline]
    pub fn import_stmt(&self, i: u32) -> &[Import] {
        &self.local_imports[self.import_stmts[i as usize].range()]
    }
    #[inline]
    pub fn case_list(&self, l: ListRef) -> &[CaseClause] {
        &self.cases[l.range()]
    }

    /// Whether a case of the list has a malformed pattern or guard (`broken_cases`).
    pub fn broken_cases_in(&self, l: ListRef) -> bool {
        let from = self.broken_cases.partition_point(|&i| i < l.start);
        self.broken_cases.get(from).is_some_and(|&i| i < l.start + l.len)
    }
    #[inline]
    pub fn try_expr(&self, i: u32) -> TryExpr {
        self.tries[i as usize]
    }
    #[inline]
    pub fn param_annots(&self, p: &Param) -> &[Annot] {
        &self.param_annots[p.annots.range()]
    }
    #[inline]
    pub fn annot_args(&self, a: &Annot) -> &[StrId] {
        &self.str_lists[a.args.range()]
    }

    /// The `new` of an annotation: the class path it names, without type arguments, and its
    /// first argument list.
    pub fn annot_new(&self, a: &Annot) -> Option<(TyExprId, ListRef)> {
        let mut e = a.instance;
        while let Expr::Apply(f, _) = self.expr(e) {
            e = f;
        }
        let Expr::New(ty, args) = self.expr(e) else { return None };
        match self.ty(ty) {
            TyExpr::Apply(path, _) => Some((path, args)),
            _ => Some((ty, args)),
        }
    }

    pub fn add_expr(&mut self, e: Expr, span: Span) -> ExprId {
        let id = ExprId(self.exprs.len() as u32);
        self.exprs.push(e);
        self.expr_spans.push(span);
        id
    }
    pub fn add_pat(&mut self, p: Pat, span: Span) -> PatId {
        let id = PatId(self.pats.len() as u32);
        self.pats.push(p);
        self.pat_spans.push(span);
        id
    }
    pub fn add_ty(&mut self, t: TyExpr, span: Span) -> TyExprId {
        let id = TyExprId(self.tys.len() as u32);
        self.tys.push(t);
        self.ty_spans.push(span);
        id
    }
    pub fn add_def(&mut self, d: Def) -> DefId {
        let id = DefId(self.defs.len() as u32);
        self.defs.push(d);
        self.def_ranges.push(NO_RANGE);
        id
    }
    /// The source spans of the parts `parts`, an interpolation's.
    pub fn set_part_spans(&mut self, parts: ListRef, spans: &[Span]) {
        let end = parts.start as usize + spans.len();
        if self.str_list_spans.len() < end {
            self.str_list_spans.resize(end, Span::default());
        }
        self.str_list_spans[parts.start as usize..end].copy_from_slice(spans);
    }

    /// Where the source writes the part at `i` of `str_lists`, when it is an interpolation's.
    pub fn part_span(&self, i: usize) -> Option<Span> {
        self.str_list_spans.get(i).copied().filter(|s| *s != Span::default())
    }

    pub fn add_str(&mut self, s: String) -> StrId {
        let id = StrId(self.strings.len() as u32);
        self.strings.push(s);
        id
    }
}

/// The ASTs of every file, by `FileId`. The typer borrows the store for its whole life and reads
/// it through `&Ast` references that it never keeps beyond a call; watch mode replaces the AST of
/// an edited file between two typing steps, when no such reference is live, which is what the
/// interior mutability is for. The typer adds the AST of a library body it converted behind the
/// program's files; each of those sits in its own box, so that the references into the others
/// stay valid while the table grows.
pub struct Asts {
    slots: Vec<std::cell::UnsafeCell<Ast>>,
    /// The ASTs of the library bodies converted during typing, pushed under the loader's lock
    /// (`shared.rs`: a push moves no entry, and a reader takes no lock).
    bodies: crate::shared::SlabVec<Box<Ast>>,
}

// Shared between the compiler thread and the typing thread (`typer/thread.rs`), which never
// touch it at once: the compiler thread blocks while a phase runs, and a replacement (watch
// mode, between two typing steps) or a push (a library body converted) happens on whichever
// thread holds the typer then. The parallel typer's discipline is its loader's lock.
unsafe impl Sync for Asts {}

impl Asts {
    /// The bytes the syntax trees hold, the library bodies' among them.
    pub fn held(&self) -> usize {
        // SAFETY: as every read of the trees, between two typing steps.
        unsafe { self.slots.iter().map(|a| (*a.get()).held()).sum::<usize>() + self.bodies.as_slice().iter().map(|a| a.held()).sum::<usize>() }
    }

    /// The library bodies converted while typing, by count and bytes.
    pub fn bodies(&self) -> (usize, usize) {
        let bodies = self.bodies.as_slice();
        (bodies.len(), bodies.iter().map(|a| a.held()).sum())
    }

    pub fn new(asts: Vec<Ast>) -> Asts {
        Asts { slots: asts.into_iter().map(std::cell::UnsafeCell::new).collect(), bodies: crate::shared::SlabVec::with_capacity(16) }
    }

    pub fn len(&self) -> usize {
        self.slots.len() + self.bodies.len()
    }

    pub fn replace(&self, i: usize, ast: Ast) -> Ast {
        unsafe { std::mem::replace(&mut *self.slots[i].get(), ast) }
    }

    /// Adds the AST of a library body behind the files: its id.
    pub fn push(&self, ast: Ast) -> usize {
        self.slots.len() + self.bodies.push(Box::new(ast))
    }

    pub fn into_vec(self) -> Vec<Ast> {
        self.slots.into_iter().map(std::cell::UnsafeCell::into_inner).collect()
    }
}

impl std::ops::Index<usize> for Asts {
    type Output = Ast;
    #[inline]
    fn index(&self, i: usize) -> &Ast {
        if i < self.slots.len() {
            unsafe { &*self.slots[i].get() }
        } else {
            self.bodies.get(i - self.slots.len())
        }
    }
}

pub fn push_list<T: Copy>(pool: &mut Vec<T>, items: &[T]) -> ListRef {
    let start = pool.len() as u32;
    pool.extend_from_slice(items);
    ListRef { start, len: items.len() as u32 }
}
