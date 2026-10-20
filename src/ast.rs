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
    /// `l op r`, the operator's end last (`NO_END` in a tree read from TASTy): where the function
    /// tree `Desugar.binop` makes of a left-associative operator (`Select(l, op)`) ends, which
    /// `Migrations.checkParentheses` reads.
    Infix(ExprId, Name, ExprId, u32),
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
    /// A sequence literal a pickle holds on its own (`SeqLiteral`): the elements of a repeated
    /// parameter's argument that a call's plan bound (`Lifter.liftArg`), of the element type.
    SeqLit(ListRef, TyExprId),
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
    /// `e _`, Scala 2's eta-expansion written out (dotty's `PostfixOp(e, Ident(_))`, typed by
    /// `Migrations.typedAsFunction`).
    AsFunction(ExprId),
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
    /// The parameter of a `{ case ... }` literal, whose body matches on it: dotty's
    /// `makeCaseLambda`, the parameter named as its synthetic one is (`x$1`).
    pub cases: bool,
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
    /// `pat <- e`, with what the version where the parser stood does to a pattern that may fail.
    Gen(PatId, ExprId, GenMode),
    /// `case pat <- e`: elements the pattern does not match are filtered out.
    CaseGen(PatId, ExprId),
    Guard(ExprId),
    Val(PatId, ExprId),
}

/// What a generator without `case` does to a pattern that may fail to match (dotty's `GenCheckMode`,
/// which `Parsers.generatorRest` picks by the version where the parser stands): before 3.2 the
/// elements it does not match are filtered out unless its syntax shows it irrefutable
/// (`FilterNow`); in 3.2 and 3.3 that, and the pattern checked against the elements' type
/// (`CheckAndFilter`); from 3.4 it is checked alone (`Check`), and a failing match throws.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GenMode {
    FilterNow,
    CheckAndFilter,
    Check,
}

impl GenMode {
    pub fn of(source: crate::source_version::SourceVersion) -> GenMode {
        use crate::source_version::SourceVersion as V;
        if source.is_at_least(V::V3_4) {
            GenMode::Check
        } else if source.is_at_least(V::V3_2) {
            GenMode::CheckAndFilter
        } else {
            GenMode::FilterNow
        }
    }
}

/// Which of dotty's `makePatDef` optimizations a tuple pattern definition takes (`Ast::patdef_tuple`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatDefTuple {
    Simple,
    Matchable,
}

/// A component of a tuple pattern definition: its variable (none for a wildcard), the type written
/// for it, where it is.
#[derive(Clone, Copy, Debug)]
pub struct TupleComponent {
    pub name: Option<Name>,
    pub ty: Option<TyExprId>,
    pub span: Span,
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
    /// The type of the `n`th parameter of the `@main` method `def` of the same tree, as its typed
    /// signature has it (`parser::add_main_proxies`: dotty's `MainProxies` gives `parseArgument` the
    /// formal parameter types).
    MainParam(DefId, u32),
    /// A lowercase type argument of a pattern type (`case l: List[t]`): a type variable the
    /// case binds. An `inline match` binds it to what it matched, an ordinary match reads it
    /// as a wildcard.
    TypeVar(Name),
    /// `T @unchecked`: a type test against it is not checked for being decidable.
    Unchecked(TyExprId),
    /// `T @uncheckedVariance`: left out of the variance check.
    UncheckedVariance(TyExprId),
    /// `T @A`, the annotation's class path as written whatever its spelling (an import may rename
    /// it), which the typer resolves: `scala.annotation.internal.RuntimeChecked`, the annotation of
    /// `Predef.runtimeChecked`'s result, is the type a pattern's static checks exempt, any other
    /// leaves `T` as it is (`None`: read from TASTy, where the class was checked by its full
    /// name). A wildcard inside stands for the ascribed expression's own type (`e: @A`). The
    /// annotations read by their name (`@unchecked`, `@uncheckedVariance`, `@nowarn`) are not kept.
    Annotated(TyExprId, Option<TyExprId>),
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
    /// The setter `x_=` of a `var x`: of an abstract var (with `ABSTRACT`), the member an
    /// assignment through the var calls and an implementing class defines; of a concrete one,
    /// a member whose call is the var's assignment (`typer::setters`). The namer makes it,
    /// without a definition, or the loader reads it from a pickle.
    pub const SETTER: Mods = 1 << 25;
    /// A class a Java class or interface declares as a static member (every member class of an
    /// interface, those marked `static` in a class): no inner class, its type through no prefix,
    /// as dotty enters it in the Java class's companion.
    pub const JAVA_STATIC: Mods = 1 << 26;
    /// A case class's selector `_N` (dotty's `Desugar.productElemMeths`): a def reading the N-th
    /// parameter of the first clause, which the namer makes without a definition and whose
    /// signature the field's gives (`complete_sig_inner`), its body the class's check's.
    pub const SELECTOR: Mods = 1 << 29;
    /// A deferred given: `given x: T = deferred` in a trait, the marker resolving to
    /// `scala.compiletime.deferred` (scalac's `Deferred | HasDefault`, set as its signature
    /// completes), or one read from a pickle with `HASDEFAULT` and no body. Abstract, and
    /// implemented by every class that is the first to extend its trait, by a search where the
    /// class is defined (`Worker::implement_deferred_givens`).
    pub const DEFERRED: Mods = 1 << 27;
    /// The definition is annotated `@deprecated` (a program's) or its pickle or class file says
    /// it is deprecated (a library's): its references warn (`typer::deprecation`).
    pub const DEPRECATED: Mods = 1 << 28;
    /// A class's: a trait that extends `Any` (scalac's universal trait, which derives from no
    /// `Object`, so is no `AnyRef`; dotty's `Namer.ensureFirstIsClass` puts `Object` before a
    /// trait's first parent otherwise): its first parent written `Any`, as a jar's pickle declares
    /// it, and Java's `Serializable` and `Cloneable` (`ClassfileParser.parseParents`).
    pub const UNIVERSAL: Mods = 1 << 30;
    /// A temporary an application's plan binds in front of the call (dotty's
    /// `EtaExpansion.Lifter`): the receiver, an argument or a default getter's application,
    /// evaluated once where the plan has it (`typer::elaborate`).
    pub const LIFTED: Mods = 1 << 31;
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
    /// Where each of `args` ends, its `)` (none in a tree read from TASTy).
    pub ends: Vec<u32>,
}

impl Parent {
    /// Where the function tree the `k`th argument list applies ends, dotty's `tree.span.end` of
    /// `New(T)` or of the application of the lists before it (`Migrations.checkParentheses`).
    pub fn fun_end(&self, ast: &Ast, k: usize) -> u32 {
        match k {
            0 => ast.ty_end(self.ty),
            _ => self.ends.get(k - 1).copied().unwrap_or(NO_END),
        }
    }
}

/// No place in the source: a tree read from TASTy, an argument list the compiler made.
pub const NO_END: u32 = u32::MAX;

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
    /// A structural instance's parents where one is a constructor call (`given t: P(1) with Q:`),
    /// its arguments the instance's superclass call (dotty's `Parsers.givenDef` template over
    /// `constrApps`); empty for `given t: T with { .. }`, whose one parent is `ty`.
    pub parents: Vec<Parent>,
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

/// An `extension` group's export clauses (`Namer.processExports`' `ExtMethods` case): the starts of
/// their clauses (`Import::span`), the group's type parameters and leading parameter clauses, and
/// the group's methods, among which the qualifier names the companion extension method
/// (`exportPathSym`).
#[derive(Clone, Debug)]
pub struct ExtensionExport {
    pub starts: Vec<u32>,
    pub tparams: Vec<TypeParam>,
    pub clauses: Vec<ParamClause>,
    pub methods: Vec<DefId>,
}

impl Ast {
    /// The extension group whose export clause starts at `start`, where one does.
    pub fn extension_export_at(&self, start: u32) -> Option<&ExtensionExport> {
        self.extension_exports.iter().find(|e| e.starts.contains(&start))
    }
}

#[derive(Clone, Debug)]
pub enum ImportSel {
    Wildcard,
    Given,
    Name(Name, Option<Name>),
}

/// Whether the clause looks like a language import (dotty's `TreeInfo.languageImport`, by the
/// written path alone): `language`, `scala.language` or `_root_.scala.language`, or one of their
/// `experimental` and `deprecated` members, as the qualifier; or `scala.language` itself named.
pub fn is_language_import(imp: &Import) -> bool {
    let path = match imp.path.as_slice() {
        [crate::names::ROOT, crate::names::SCALA, rest @ ..] | [crate::names::SCALA, rest @ ..] => rest,
        rest => rest,
    };
    match path {
        [crate::names::LANGUAGE] => true,
        [crate::names::LANGUAGE, sub] => *sub == crate::names::EXPERIMENTAL || *sub == crate::names::DEPRECATED,
        _ => false,
    }
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
    /// (`typer::unused`), and which bounds the givens its clause brings (`Namer.importBound`).
    pub bound: Option<TyExprId>,
    /// Where scalac's tree of the clause starts: the `import` keyword for a statement's first
    /// clause, the clause's path for the others (`isPrimaryClause`).
    pub tree_start: u32,
    /// The end of the clause's path, the qualifier its selectors select from (`span.start`
    /// for none), and the end of the clause, past its closing brace: the positions of the
    /// unused-import edits (`typer::unused_edits`).
    pub qual_end: u32,
    pub clause_end: u32,
}

/// What the parser notes of the syntax where the migration table's checks read it, few files having any: kept
/// apart from the tree's own tables.
#[derive(Default, Debug)]
pub struct SyntaxNotes {
    /// The operators of the infix types and the infix extractor patterns written with an identifier
    /// (`A Pair B`, `case a Cons b`; not `A | B`, `case h :: t`), ascending: the `Name` an infix type's
    /// `Apply` applies and the path its `Ctor` names, which `Checking.checkValidInfix` reads as the
    /// `InfixOp` the parser kept (`Typer.typedInfixOp`).
    pub infix_type_ops: Vec<TyExprId>,
    pub infix_pattern_ops: Vec<ExprId>,
    /// `(a = 1)`: the named tuple of one element it is from 3.7, or the `Parens` node it is before
    /// (dotty's `Parens(Assign(..))`), each with the assignment written and the span of its own
    /// parentheses, ascending, which the typer reports (`Typer.checkDeprecatedAssignmentSyntax`).
    pub assignment_tuples: Vec<(ExprId, ExprId, Span)>,
    /// A named tuple of more than one element written as an infix operation's right operand, ascending,
    /// with its elements as named arguments: what an operation of a method of more than one parameter
    /// passes (`Applications.needsTupledDual` of `ApplyKind.InfixTuple`).
    pub named_operands: Vec<(ExprId, ListRef)>,
    /// The ends of the identifier tokens the scanner read backquoted (`BACKQUOTED_IDENT`, `LexResult::backquoted`),
    /// ascending: `untpd.isBackquoted` of a name, the `Backquoted` attachment `Parsers.makeIdent` pushes. The unit's
    /// record, which the parser installs on every AST it makes for the unit.
    pub backquoted_idents: Vec<u32>,
}

pub struct Ast {
    /// The companion objects the parser made, in order: `Desugar.classDef`'s of a case class or
    /// of a class with a `derives` clause that the body writes none for.
    pub synthetic_companions: Vec<DefId>,
    /// The givens of `derives` clauses the parser made, in order, which `Deriving.enterDerived`
    /// enters in the companion apart from its written body.
    pub derived_givens: Vec<DefId>,
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
    /// What the parser notes of the syntax for the migration table's checks (`SyntaxNotes`).
    pub notes: Box<SyntaxNotes>,
    /// An infix expression's operator was an identifier (`a max b`, not `a + b`): only then is one
    /// checked as `Checking.checkValidInfix` does.
    pub alpha_infix: bool,
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
    /// The `export` clauses written inside `extension` groups (`Parsers.extension`, an `Export`
    /// among the `ExtMethods`), each group's with its header: the clauses themselves stand among
    /// the enclosing body's (`ClassDef::exports`, `top_exports`).
    pub extension_exports: Vec<ExtensionExport>,
    /// `import language.strictEquality` was written in the file.
    pub strict_equality: bool,
    /// The file's top-level imports of `scala.language`, which the parser takes out of
    /// `imports`: they name the object all the same.
    pub language_imports: Vec<Import>,
    /// Each feature a `scala.language` import of the file names, whether it enables it (`f =>
    /// _` takes it back) and its scope: from the import to the end of the block or body it stands
    /// in, or of the file (`typer::feature`). A wildcard names none.
    pub language_scopes: Vec<(Name, bool, Span)>,
    /// The initialisers written `_` (`var x: T = _`), which only a field may have.
    pub default_inits: Vec<ExprId>,
    /// The names of one `val a, b = e` after the first, each with the name before it: every name
    /// has its own copy of `e` (`parser/defs.rs`), typed in this order whichever is asked for
    /// first (`typer/check.rs`).
    pub val_copies: crate::intern::FxMap<DefId, DefId>,
    /// The expressions whose parentheses or brace block the parser collapsed (`(e)`, `{ e }`, a colon
    /// argument's indented region), each with its span as written and with one level of parentheses
    /// unwrapped: dotty's argument tree where the expression is an argument (`Migrations.implicitParams`'
    /// `pt.args.head`), the second where it is an operator's operand, which `Desugar.binop` unwraps
    /// (`P f ((1))`'s `(1)`, `P f { (1) }`'s block). By expression, ascending, as the parser completes
    /// them: an expression's last entry is its outermost (`Ast::written_arg`).
    pub written_args: Vec<(ExprId, Span, Span)>,
    /// The unit's source version, which typing reads (`Feature.sourceVersion`): its source version
    /// import's, else the setting's. A package block's is its file's.
    pub source_version: crate::source_version::SourceVersion,
    /// `import language.experimental.relaxedLambdaSyntax` was met: a lambda after `:` may stand on
    /// the colon's line (dotty's `followingIsLambdaAfterColon`, case 1b).
    pub relaxed_lambdas: bool,
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
    /// The selectors of the matches a member's pattern definition makes (`val P = e`, dotty's
    /// `makePatDef` with `MatchCheck.IrrefutablePatDef`), whose case is checked to be irrefutable.
    pub patdef_scrutinees: std::collections::HashSet<ExprId, crate::intern::FxBuild>,
    /// A pattern in parentheses of its own (`(x)`, `(h: Accept)`, `((x: Int))` once), which the parser reads as the
    /// pattern: dotty's `Parens` (`Parsers.simplePattern`'s `makeTupleOrParens`), a node of the untyped trees
    /// `makePatDef` (`Ast::patdef_tuple`) and `makeFor`'s `isVarBinding` and `isIrrefutable` (`needs_no_filter`) read.
    pub paren_pats: std::collections::HashSet<PatId, crate::intern::FxBuild>,
    /// The vals a member's pattern definition derives for its variables (dotty's `derivedValDef`
    /// with its `PatternVar` attachment): pattern variables to the unused check, never reported.
    pub patdef_components: std::collections::HashSet<DefId, crate::intern::FxBuild>,
    /// The holders a pattern definition's lowering makes (`$N`, dotty's `UniqueName` holder of
    /// `patMods` that include `Synthetic`, Desugar.scala 1637-1647): an owner a macro skips as it
    /// skips dotty's synthetic symbols (`Flags.Synthetic`).
    pub patdef_holders: std::collections::HashSet<DefId, crate::intern::FxBuild>,
    pub paren_annotated: std::collections::HashSet<ExprId, crate::intern::FxBuild>,
    /// The `new C()` written with an empty first argument list before further lists: the `()`
    /// is an argument list, never the place of a using clause.
    pub new_empty_first: Vec<ExprId>,
    /// A library body's type argument lists that scalac inferred, by their start in
    /// `ty_lists`, ascending: where they do not check, the typer infers the arguments again.
    pub inferred_type_lists: Vec<u32>,
    /// A library body's cast targets whose type, as written, names a member of Scala.js's
    /// `scala.scalajs.js` package that the std holds as an alias (`js.Array[Int]`): what a
    /// source's spelling tells the typer (`written_js_alias`), which the resolved type no longer does.
    pub js_alias_types: Vec<TyExprId>,
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
    /// The annotations written on types (`Int @Ann(m)`), each with the type it annotates and
    /// the annotation as `inline_annots` reads it (`Annot::instance`), in the types' order; a
    /// library body's, those its pickle wrote (`ANNOTATEDtpt`) as the paths they name. A type's
    /// node leaves them out; dotty's `Annotated` tree keeps the annotation's tree.
    pub type_annots: Vec<(TyExprId, ExprId)>,
    /// Whether an annotation named `nowarn` was written in the file, whose definitions and
    /// ascriptions `@nowarn` suppresses the warnings of (`warnings.rs`), and the ascriptions
    /// and types so annotated (`e: @nowarn`, `T @nowarn`), each with its whole range.
    pub has_nowarn: bool,
    /// The proxies `parser::add_main_proxies` made, each with the `@main` method it runs.
    pub main_proxies: Vec<(DefId, DefId)>,
    pub nowarn_ascriptions: Vec<(Span, Annot)>,
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
    /// The expressions written alone in braces (`{ e }`), which the parser reads as `e`: scalac's
    /// tree is a block whose expression it types and adapts inside it. A bit per expression, by
    /// its id, set by the parser (`mark_braced`): the mark dotty's `Block` node is, read where a
    /// node is typed.
    pub braced: Vec<u64>,
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
    /// A product's binder the converter renamed apart where it shadows one its scope reads
    /// (`x$b117`), by that name: its own name, which its local symbol takes, as the whole
    /// program's has it, the scope resolving the renamed spelling.
    pub binder_sources: crate::intern::FxMap<Name, Name>,
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
            synthetic_companions: Vec::new(),
            derived_givens: Vec::new(),
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
            notes: Box::default(),
            alpha_infix: false,
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
            extension_exports: Vec::new(),
            strict_equality: false,
            language_imports: Vec::new(),
            language_scopes: Vec::new(),
            default_inits: Vec::new(),
            val_copies: crate::intern::FxMap::default(),
            written_args: Vec::new(),
            source_version: Default::default(),
            relaxed_lambdas: false,
            top_level: Vec::new(),
            access_scopes: Vec::new(),
            has_quotes: false,
            new_outers: crate::intern::FxMap::default(),
            given_binds: Default::default(),
            patdef_scrutinees: Default::default(),
            paren_pats: Default::default(),
            patdef_components: Default::default(),
            patdef_holders: Default::default(),
            paren_annotated: Default::default(),
            new_empty_first: Vec::new(),
            inferred_type_lists: Vec::new(),
            js_alias_types: Vec::new(),
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
            type_annots: Vec::new(),
            has_nowarn: false,
            main_proxies: Vec::new(),
            nowarn_ascriptions: Vec::new(),
            recoveries: Vec::new(),
            cut_args: Vec::new(),
            broken_cases: Vec::new(),
            braced: Vec::new(),
        }
    }

    /// Whether the definition `d`, one of its parameters or one of its type parameters is
    /// annotated.
    #[inline]
    pub fn def_has_annotations(&self, d: DefId) -> bool {
        let def = self.def(d);
        if !def.annots.is_empty() {
            return true;
        }
        let (tparams, clauses): (&[TypeParam], &[ParamClause]) = match &def.kind {
            DefKind::Fun(f) => (&f.tparams, &f.clauses),
            DefKind::Class(c) => (&c.tparams, &c.clauses),
            DefKind::Given(g) => (&g.tparams, &g.clauses),
            DefKind::TypeAlias { tparams, .. } => (tparams, &[]),
            DefKind::Val { .. } => (&[], &[]),
        };
        // The parameters walked where the file annotates one.
        tparams.iter().any(|tp| !tp.annots.is_empty()) || (!self.param_annots.is_empty() && clauses.iter().flat_map(|c| c.params.iter()).any(|p| !p.annots.is_empty()))
    }

    /// The range of the whole definition, or its name's span where the parser has none.
    /// Whether the parser made the definition, no source written for it: `Desugar.classDef`'s
    /// companion of a case class or a `derives` clause's given.
    pub fn parser_made(&self, id: DefId) -> bool {
        self.synthetic_companions.binary_search(&id).is_ok() || self.derived_givens.binary_search(&id).is_ok()
    }

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

    /// Where the type tree `t` ends, `NO_END` where its tree records no span.
    pub fn ty_end(&self, t: TyExprId) -> u32 {
        self.ty_spans.get(t.idx()).map_or(NO_END, |s| s.end)
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

    /// Whether `e` was written alone in braces (`braced`).
    #[inline]
    pub fn is_braced(&self, e: ExprId) -> bool {
        let i = e.idx();
        self.braced.get(i / 64).is_some_and(|w| w >> (i % 64) & 1 != 0)
    }

    /// `e` collapsed out of the parentheses or the block written over `written` (`written_args`): for a
    /// block both its spans, for parentheses that span and the one they enclose, an earlier entry's.
    pub fn push_written(&mut self, e: ExprId, written: Span, parens: bool) {
        let last = self.written_args.last().copied();
        let inner = match last {
            _ if !parens => written,
            Some((x, outer, _)) if x == e => outer,
            Some((x, ..)) if x > e => self.written_arg(e).map_or(self.expr_spans[e.idx()], |(outer, _)| outer),
            _ => self.expr_spans[e.idx()],
        };
        // The parser completes the expression it keeps after those within it: an entry goes last, but where
        // a tree was made out of that order.
        match last {
            Some((x, ..)) if x > e => {
                let at = self.written_args.partition_point(|&(x, ..)| x <= e);
                self.written_args.insert(at, (e, written, inner));
            }
            _ => self.written_args.push((e, written, inner)),
        }
    }

    /// The span as written of `e` (`written_args`) and the one with a level of parentheses unwrapped.
    pub fn written_arg(&self, e: ExprId) -> Option<(Span, Span)> {
        let i = self.written_args.partition_point(|&(x, ..)| x <= e);
        i.checked_sub(1).map(|i| self.written_args[i]).filter(|&(x, ..)| x == e).map(|(_, written, inner)| (written, inner))
    }

    /// `e` was written alone in braces.
    pub fn mark_braced(&mut self, e: ExprId) {
        let i = e.idx();
        if self.braced.len() <= i / 64 {
            self.braced.resize(i / 64 + 1, 0);
        }
        self.braced[i / 64] |= 1 << (i % 64);
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

    /// dotty's `makePatDef` optimizations of a tuple pattern definition (Desugar.scala 1505-1660):
    /// a tuple pattern whose every element is a variable, a wildcard or one of them typed (not in
    /// parentheses of its own), its components in order, over a right side whose every result is a
    /// tuple of that arity (`SimpleTuple`: the right side as it is) or any other but a `: @unchecked`
    /// one (`MatchableTuple`: `rhs match { case $1 @ pat => $1 }`). Each named component is then a
    /// val of the element, of the type written for it: a typed element is a declared type there,
    /// checked by typing, which the definition's refutability check does not exempt.
    pub fn patdef_tuple(&self, pat: PatId, rhs: ExprId) -> Option<(PatDefTuple, Vec<TupleComponent>)> {
        let Pat::Tuple(l) = self.pat(pat) else { return None };
        let pats = self.pat_list(l);
        let mut components = Vec::with_capacity(pats.len());
        for &p in pats {
            let span = self.pat_spans[p.idx()];
            let (inner, ty) = match self.pat(p) {
                Pat::Typed(inner, t) if !self.paren_pats.contains(&p) => (inner, Some(t)),
                Pat::Typed(..) => return None,
                _ => (p, None),
            };
            let name = match self.pat(inner) {
                Pat::Bind(n, None) => Some(n),
                Pat::Wildcard => None,
                _ => return None,
            };
            components.push(TupleComponent { name, ty, span });
        }
        let arity = components.len();
        // `isMatchingTuple`: a tuple literal of the pattern's arity without a named argument.
        let matching = |e: ExprId| match self.expr(e) {
            Expr::Tuple(l) => l.len as usize == arity && !self.expr_list(l).iter().any(|&x| matches!(self.expr(x), Expr::NamedArg(..))),
            _ => false,
        };
        if self.all_results(rhs, &matching) {
            Some((PatDefTuple::Simple, components))
        } else if (matches!(self.expr(rhs), Expr::Unchecked(_)) || self.annotation_only(rhs)) && !self.paren_annotated.contains(&rhs) {
            // `!rhs.isInstanceOf[Annotated]` (Desugar.scala 1582): any annotated right side, not
            // in parentheses of its own, takes the general match.
            None
        } else {
            Some((PatDefTuple::Matchable, components))
        }
    }

    /// The type under `t`'s annotations (`T @A @B`'s `T`), whose own shape a reader of the written
    /// type asks for.
    pub fn under_annotations(&self, mut t: TyExprId) -> TyExprId {
        while let TyExpr::Annotated(inner, _) = self.ty(t) {
            t = inner;
        }
        t
    }

    /// Whether `e` is written `: @unchecked`, under any other annotations ascribed over it
    /// (`e: @unchecked @A`, an annotation-only ascription a layer each): dotty's
    /// `hasAnnotation(UncheckedAnnot)` on the typed tree's type, which looks through stacked
    /// annotations (Types.scala 438-440), the one query refutability (Checking.scala 1146),
    /// exhaustivity and reachability (Space.scala 947, 1111) make.
    pub fn is_unchecked(&self, mut e: ExprId) -> bool {
        loop {
            match self.expr(e) {
                Expr::Unchecked(_) => return true,
                Expr::Typed(inner, _) if self.annotation_only(e) => e = inner,
                _ => return false,
            }
        }
    }

    /// Whether `e` is an annotation-only ascription, dotty's `Annotated(e, annot)` tree (`e: @A`).
    pub fn annotation_only(&self, e: ExprId) -> bool {
        let Expr::Typed(_, mut t) = self.expr(e) else { return false };
        let mut wrapped = false;
        while let TyExpr::Annotated(inner, _) = self.ty(t) {
            t = inner;
            wrapped = true;
        }
        wrapped && matches!(self.ty(t), TyExpr::Wildcard)
    }

    /// Whether `e` is an ascription to a written type (`e: T`, dotty's `Typed`, of the type written),
    /// not an annotation-only one (`e: @A`, `e`'s own type annotated).
    pub fn is_ascription(&self, e: ExprId) -> bool {
        matches!(self.expr(e), Expr::Typed(..)) && !self.annotation_only(e)
    }

    /// The type under `t`'s annotations and their class paths, outermost first.
    pub fn annotations_of(&self, mut t: TyExprId) -> (TyExprId, Vec<Option<TyExprId>>) {
        let mut classes = Vec::new();
        while let TyExpr::Annotated(inner, class) = self.ty(t) {
            classes.push(class);
            t = inner;
        }
        (t, classes)
    }

    /// dotty's `forallResults` (TreeInfo.scala 355-363) over the untyped right side `makePatDef`
    /// reads, whole: `p` of each result of an `If`, a `Match`, a `Block` and `Parens`, and of every
    /// tree dotty makes a subclass of one (Trees.scala: `InlineIf` of `If`, 588; `InlineMatch` and
    /// `SubMatch` of `Match`, 615 and 621; untpd's `XMLBlock` of `Block`, untpd.scala 182), which
    /// teq writes as `Expr::InlineIf` and `Expr::InlineMatch` (no sub-match or XML literal is
    /// parsed). A `try` is the parser's `ParsedTry` there, which `forallResults`' `Try` case does not
    /// match, so it is a result, as is anything else.
    fn all_results(&self, e: ExprId, p: &dyn Fn(ExprId) -> bool) -> bool {
        match self.expr(e) {
            Expr::If(_, t, Some(f)) | Expr::InlineIf(_, t, Some(f)) => self.all_results(t, p) && self.all_results(f, p),
            Expr::Match(_, cases) | Expr::InlineMatch(_, cases) => self.case_list(cases).iter().all(|c| self.all_results(c.body, p)),
            Expr::Block(stmts) => match self.stmt_list(stmts).last() {
                Some(Stmt::Expr(r)) => self.all_results(*r, p),
                _ => false,
            },
            Expr::Parens(inner) => self.all_results(inner, p),
            _ => p(e),
        }
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
    #[inline]
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
