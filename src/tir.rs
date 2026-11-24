//! Typed intermediate representation: what the typer produces and the JS emitter consumes.
//! All name resolution, implicit arguments, extension calls and for-comprehensions are already
//! elaborated here.

use crate::arena::{Arena, Bits, Parallel, Registers};
use crate::ast::{mods, ListRef};
use crate::intern::{FxMap, Name};
use crate::source::{FileId, Span};
use crate::types::{ClassId, SymId, TParamId, TypeId};
use crate::symbols::{Owner, SymKind, Symbols};

pub mod capture;

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
id_type!(TExprId, TPatId, StrRef, FunId, TestId);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimOp {
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntRem,
    IntAnd,
    IntOr,
    IntXor,
    IntShl,
    IntShr,
    IntUshr,
    LongAdd,
    LongSub,
    LongMul,
    LongDiv,
    LongRem,
    LongAnd,
    LongOr,
    LongXor,
    LongShl,
    LongShr,
    LongUshr,
    DoubleAdd,
    DoubleSub,
    DoubleMul,
    DoubleDiv,
    DoubleRem,
    FloatAdd,
    FloatSub,
    FloatMul,
    FloatDiv,
    FloatRem,
    Lt,
    Le,
    Gt,
    Ge,
    /// Strict JS equality, valid for primitives and reference identity.
    RefEq,
    RefNe,
    /// Structural equality through the runtime.
    Eq,
    Ne,
    /// `&&`: the right operand is evaluated only when the left is true.
    BoolAnd,
    /// `||`: the right operand is evaluated only when the left is false.
    BoolOr,
    BoolXor,
    /// `&` of a `Boolean`, an ordinary method in scalac: both operands, the left first.
    BoolStrictAnd,
    /// `|` of a `Boolean`: both operands, the left first.
    BoolStrictOr,
    /// `eq`, reference identity (scalac's `ID`, `Object.eq`): a string or a box compares as a
    /// reference, where `RefEq` of two strings compares their values. Last, so that the
    /// discriminants before them, which the outline hashes encode, stay what they were.
    Same,
    /// `ne`.
    NotSame,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnOp {
    IntNeg,
    LongNeg,
    DoubleNeg,
    FloatNeg,
    BoolNot,
    IntNot,
    LongNot,
    IntToLong,
    LongToDouble,
    LongToInt,
    DoubleToInt,
    DoubleToLong,
    CharToInt,
    CharToLong,
    IntToChar,
    /// A `Byte` and a `Short` are ints in the range of their type: the narrowing wraps.
    IntToByte,
    IntToShort,
    IntToFloat,
    LongToFloat,
    FloatToInt,
    FloatToLong,
    FloatToDouble,
    DoubleToFloat,
    /// A widening on the JVM and in the interpreter, the same number on JavaScript
    /// (`same_number`). Every widening between two kinds of box has a node: the JVM unboxes the
    /// operand as its own kind before it widens (`xs.head.toDouble` of a `List[Int]`). Last, so
    /// that the discriminants before them, which the outline hashes encode, stay what they were.
    IntToDouble,
    ByteToShort,
    ByteToInt,
    ShortToInt,
}

impl UnOp {
    /// A widening whose result is the same JavaScript number as its operand: the emitter writes
    /// the operand and sees the operand's structure through it.
    pub fn same_number(self) -> bool {
        matches!(self, UnOp::IntToDouble | UnOp::ByteToShort | UnOp::ByteToInt | UnOp::ShortToInt)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrKind {
    /// Already a JS string (String and Char).
    Str = 0,
    /// Int and Boolean: JS string conversion matches Scala.
    Plain = 1,
    Double = 2,
    Long = 3,
    /// Anything else goes through the runtime `$str`.
    Generic = 4,
}

/// How the operand of a `ToStr` becomes a string and who asked for it, in the one byte the
/// kind took: a wider node costs the interpreter's dispatch 0.5% on a program of macros.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StrConv(u8);

impl StrConv {
    const CALL: u8 = 8;

    /// What a concatenation or an interpolation makes of an operand that is no string: scalac
    /// evaluates every operand of the chain before it renders the first.
    #[inline]
    pub fn rendering(kind: StrKind) -> StrConv {
        StrConv(kind as u8)
    }

    /// The program's `x.toString`: part of the evaluation of the operand it stands in.
    #[inline]
    pub fn call(kind: StrKind) -> StrConv {
        StrConv(kind as u8 | Self::CALL)
    }

    #[inline]
    pub fn is_rendering(self) -> bool {
        self.0 & Self::CALL == 0
    }

    /// The byte, which tells a call from a rendering of the same kind.
    #[inline]
    pub fn bits(self) -> u8 {
        self.0
    }

    #[inline]
    pub fn kind(self) -> StrKind {
        match self.0 & !Self::CALL {
            0 => StrKind::Str,
            1 => StrKind::Plain,
            2 => StrKind::Double,
            3 => StrKind::Long,
            _ => StrKind::Generic,
        }
    }
}

impl std::fmt::Debug for StrConv {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}({:?})", if self.is_rendering() { "Rendering" } else { "Call" }, self.kind())
    }
}

/// The tag is a byte of its own, first (`repr(u8)`): no variant's field lends the tag a niche, which
/// would make every match on a node decode it (the interpreter's dispatch).
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TExpr {
    Int(i32),
    Long(i64),
    Double(f64),
    Bool(bool),
    Char(u16),
    Str(StrRef),
    Unit,
    Local(SymId),
    This,
    /// The receiver of a `super` call: `this`, with the method chosen as the target says.
    Super(SuperTarget),
    /// A top-level or object-member val, or an enum value.
    Static(SymId),
    Module(ClassId),
    Field(TExprId, SymId),
    CallStatic(SymId, ListRef),
    CallMethod(TExprId, SymId, ListRef),
    CallClosure(TExprId, ListRef),
    New(ClassId, ListRef),
    /// An instance made through a secondary constructor, the `<init>` symbol of its class.
    NewVia(SymId, ListRef),
    /// Params live in `Program::sym_lists`.
    Lambda(ListRef, TExprId),
    If(TExprId, TExprId, Option<TExprId>),
    While(TExprId, TExprId),
    /// Statements live in `Program::stmts`; the expression is the block's value.
    Block(ListRef, TExprId),
    Assign(TExprId, TExprId),
    /// Cases live in `Program::cases`.
    Match(TExprId, ListRef),
    Prim(PrimOp, TExprId, TExprId),
    Unary(UnOp, TExprId),
    StrConcat(ListRef),
    ToStr(TExprId, StrConv),
    Js(StrRef, ListRef),
    /// `e.isInstanceOf[T]`: the test of `e`, and the type tested as the typer resolved it, which a
    /// pattern's test carries too (`TPat::Test`): the JVM's test of an array reads the element there.
    TypeTest(TExprId, TestId, TypeId),
    /// `e.asInstanceOf[T]` that does something at run time, `T` as written: the operation dotty's
    /// erasure leaves of the cast (`TypeTestsCasts.transformAsInstanceOf`), the operand evaluated
    /// first. A cast its erasure makes redundant is the operand itself, retyped as an ascription is.
    Cast(TExprId, CastOp, TypeId),
    /// `classOf[C]`: the class value of `C`, a builtin included; with dimensions, of the array of
    /// that many of `C` (`classOf[Array[Int]]` is `Int`'s of one), as the erasure `Erased::Of`
    /// says it. The array class of no dimensions is an array's whose element is not known.
    ClassOf(ClassId, u8),
    /// Varargs packed into a runtime sequence.
    SeqLit(ListRef),
    /// Raw JS array and element access, used for compiler-internal packing.
    ArrayLit(ListRef),
    Index(TExprId, u32),
    /// The module-level binding of `Program::js_imports[i]`.
    JsImport(u32),
    /// A global JS identifier; guarded reads give `undefined` for an undeclared one.
    JsGlobal(Name, bool),
    /// A property of a JS value under its verbatim name (`js.Dynamic` sugar).
    JsSelect(TExprId, Name),
    /// A JS object literal: keys (as `Str`) and values alternate in the list.
    ObjLit(ListRef),
    /// An argument that is spread into a JS call: `...xs`.
    Spread(TExprId),
    /// `return e` from the enclosing method, through the lambdas in between.
    Return(TExprId),
    Null,
    /// `throw e`; the flag says that `e` may be a `JavaScriptException`, whose raw value is
    /// what is thrown then.
    Throw(TExprId, bool),
    /// An index into `Program::tries`.
    Try(u32),
    /// The application of a parameter's default getter, where the source left the argument out:
    /// dotty's `f$default$N` selected on the call's receiver and applied to the arguments of the
    /// clauses before the parameter's (`Applications.defaultArgument`, `findDefaultGetter`,
    /// `spliceMeth`). The `u32` is the parameter's place among the callee's, counted across its
    /// clauses, with `DEFAULT_RECEIVER` set where the list starts with the receiver; the
    /// arguments of the earlier clauses follow. The getter is the receiver's class's: the one of
    /// the nearest override that declares the default.
    Default(DefaultOf, u32, ListRef),
    /// `${ e }` at the top of an inline method's body typed at its definition: a `Lambda` of the
    /// `Quotes` the macro's code `e` takes, `Quotes ?=> Expr[T]` for the node's type `T`, which
    /// runs at each expansion with that expansion's arguments. Only a stored
    /// inline body holds one, and no backend reads a stored body.
    Splice(TExprId),
}

/// Whose default getter a `TExpr::Default` applies: a method's, a secondary constructor's
/// `<init>` included, or a class's primary constructor's, whose getters its companion has.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DefaultOf {
    Method(SymId),
    Ctor(ClassId),
}

/// The bit of a `TExpr::Default`'s index that says its list starts with the receiver.
pub const DEFAULT_RECEIVER: u32 = 1 << 31;

/// A `TExpr` stays three words: a wider node costs the interpreter's dispatch.
const _: () = assert!(std::mem::size_of::<TExpr>() == 24);

/// What a cast does, decided on the erased types of its operand and its target
/// (`Worker::cast_lowering`), as `TypeTestsCasts.transformAsInstanceOf` and `Erasure.Boxing` decide it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CastOp {
    /// Not decided yet: a cast of a stored inline body or of a quote, decided where the body is
    /// instantiated with the types that fill it.
    Written,
    /// The test of the target's erasure, which `null` passes; any other value that fails it throws
    /// `ClassCastException` (the JVM's `checkcast`). The type is that erasure, as a class type or
    /// an array of one: the destination the JVM checks, decided once with the test.
    Check(TestId, TypeId),
    /// The unboxing to the primitive the test is of (`BoxesRunTime.unboxToInt`): `null` is the
    /// primitive's zero, a value that fails the test throws `ClassCastException`. The type is
    /// the primitive's.
    Unbox(TestId, TypeId),
    /// The erasure's unboxing of a value a call with an erased result gave, where its primitive
    /// type is consumed (`Worker::unbox_erased`, dotty's `Erasure.Boxing.adaptToType`): `null` is
    /// the primitive's zero, as `Unbox`; on JavaScript any other value is taken as it is
    /// (unchecked, as Scala.js's production unboxing), a field read before its assignment
    /// (`undefined`) the zero too, which the JVM reads there. Typed as `Unbox` is.
    Adapt(TestId, TypeId),
    /// A cast to `Nothing`, which throws `ClassCastException` whatever the value, `null` included.
    Nothing,
    /// A cast whose operand's erasure conforms to the type's, which erasure drops
    /// (`TypeTestsCasts.interceptTypeApply`): the operand on every target, the node keeping the
    /// type it was cast to, a synthesized mirror's refinement (`Worker::mirror_cast`).
    Erased,
}

#[derive(Clone)]
pub struct TTry {
    pub body: TExprId,
    /// Cases in `Program::cases`; a value no case takes is thrown again.
    pub cases: ListRef,
    pub finalizer: Option<TExprId>,
    /// Whether a caught value that is no `Throwable` is wrapped in a `JavaScriptException`
    /// before the cases see it, which is needed only where a case could take one.
    pub wraps: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SuperTarget {
    /// The first definition along the superclass chain, which is JavaScript's `super`.
    Chain,
    /// The definition of this class or trait.
    Class(ClassId),
    /// `super` inside this trait: the definition that follows the trait in the linearisation of
    /// the class that mixes it in, which that class names in `TClass::super_accessors`.
    Mixin(ClassId),
}

/// What `super.member` inside the trait `of_trait` stands for in one class that mixes it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuperAccessor {
    pub of_trait: ClassId,
    pub member: SymId,
    /// `None` for the `toString`, `hashCode` or `equals` of `Any`; of a getter's accessor, the
    /// member that declares the default.
    pub target: Option<SymId>,
    /// The accessor of `super.member$default$N`, the getter of the parameter at this index of
    /// `member`, counted across its clauses (`SuperAccessors` gives a getter selected on `super`
    /// an accessor of its own).
    pub getter: Option<u32>,
}

/// What a trait calls through `super`: a member, or the getter of the default of one of its
/// parameters, which a `super` call leaving the argument out applies (`super.f$default$N`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SuperCall {
    pub member: SymId,
    /// The parameter's index, counted across the member's clauses.
    pub getter: Option<u32>,
}

impl SuperCall {
    pub fn member(member: SymId) -> SuperCall {
        SuperCall { member, getter: None }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TStmt {
    Expr(TExprId),
    Val(SymId, TExprId),
    Fun(FunId),
    /// Pattern definition; the bound variables stay visible in the enclosing block. The form is the branch of
    /// dotty's `makePatDef` the typer took for it.
    Pat(TPatId, TExprId, PatDefForm),
}

/// The branch of dotty's `makePatDef` (`Desugar.scala` 1505-1673 at 3.8.4) a pattern definition takes, as the typer
/// reads the written pattern and right side (`Ast::patdef_tuple`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatDefForm {
    /// `IdPattern`: a variable alone, typed or not.
    Id,
    /// `SimpleTuple`: a tuple of variables over a right side whose every result is a tuple literal of its arity.
    SimpleTuple,
    /// `MatchableTuple`: a tuple of variables, `rhs match { case $t @ (x1, .., xn) => $t }`.
    MatchableTuple,
    /// The general match (`None`): `rhs match { case pat => (x1, .., xn) }`; an annotated right side
    /// (`: @unchecked`) and a parenthesised typed element among them.
    General,
    /// A for-comprehension's alias: the general match over `x$1 @ pat` (`makeIdPat` in `makeFor`), whose binder
    /// the typer does not make.
    ForAlias,
}

#[derive(Clone, Copy, Debug)]
pub struct TCase {
    pub pat: TPatId,
    pub guard: Option<TExprId>,
    pub body: TExprId,
}

#[derive(Clone, Copy, Debug)]
pub enum TPat {
    Wildcard,
    Bind(SymId, Option<TPatId>),
    /// A type test, the type it stands for, and the pattern that follows it.
    Test(TestId, TypeId, TPatId),
    /// Compared with `===` when the flag is set, through runtime equality otherwise.
    Equals(TExprId, bool),
    /// Class check followed by field sub-patterns; fields live in `Program::sym_lists`. The
    /// type is the class as instantiated for the scrutinee.
    Class(ClassId, TypeId, ListRef, ListRef),
    Alt(ListRef),
    /// `Seq(a, b, rest*)`: patterns for the first elements, and for the rest of the sequence.
    Seq(ListRef, Option<TPatId>),
    /// `Obj(p)` through an `unapply`: the scrutinee is bound to the local, the call is made,
    /// and the pattern is matched against its result.
    Unapply(SymId, TExprId, TPatId),
}

/// The kinds of primitive whose box extends a JDK class: `java.lang.String`, the numeric boxes,
/// `Long`, `Boolean`, `Character` and `BoxedUnit`, Scala.js's hijacked classes, whose ancestors'
/// instance tests take the primitive by its kind (`Class.isInstance` too). A bit set.
pub mod boxed {
    pub const STR: u8 = 1;
    pub const NUMBER: u8 = 2;
    pub const LONG: u8 = 4;
    pub const BOOL: u8 = 8;
    pub const CHAR: u8 = 16;
    pub const UNIT: u8 = 32;

    /// The primitives whose box extends the class `name` of package `pkg` besides `Object`, as the
    /// JDK declares the boxes (`String implements Serializable, Comparable<String>, CharSequence`;
    /// `BoxedUnit` is `Serializable` alone).
    pub fn ancestor(pkg: &str, name: &str) -> u8 {
        match (pkg, name) {
            ("java.lang", "CharSequence") => STR,
            ("java.lang", "Comparable") => STR | NUMBER | LONG | BOOL | CHAR,
            ("java.io", "Serializable") => STR | NUMBER | LONG | BOOL | CHAR | UNIT,
            ("java.lang", "Number") => NUMBER | LONG,
            _ => 0,
        }
    }

    /// The kinds of `ancestor` that pass its test on JavaScript, where a primitive's box is the
    /// primitive, as Scala.js's hijacked classes declare them: `()`'s class there extends `Object`
    /// alone (no `Serializable`), and a `Char` is a string, whose kind is `STR`.
    pub fn on_js(kinds: u8) -> u8 {
        kinds & !(UNIT | CHAR)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TypeTest {
    Always,
    Class(ClassId),
    Trait(ClassId),
    Number,
    Int,
    Array,
    Long,
    Byte,
    Short,
    Float,
    Str,
    /// A `Char` on the JVM; the JS backend, where a char is a string, is never given one.
    Char,
    Bool,
    Unit,
    /// A function of exactly this many parameters.
    Function(u8),
    Null,
    /// `AnyRef`'s and `Object`'s test, which a boxed value passes: everything but `null`, as `AnyVal`'s.
    AnyRef,
    /// The test of a type that erases to `Object` under scalac (`AnyVal`, `Any`, `Matchable`, a type parameter
    /// bounded by `Any`): everything but `null`.
    AnyVal,
    /// Equality with a literal, for a literal type.
    Value(TExprId),
    Or(TestId, TestId),
    And(TestId, TestId),
    /// The test of the scrutinee's enclosing instance, read through the outer accessor of its
    /// class (the second test applies to what the accessor returns), after a test that the
    /// scrutinee is of that class: dotty's `PatternMatcher.addOuterTest`, `x.$outer eq p` for a
    /// class through the path `p`, the outer's class test for a projection `P#C`.
    Outer(SymId, TestId),
    /// A class test the scrutinee's type passes and its erasure does not (`case _: Box[T]` of a
    /// `Box[T]`, dotty's `TypeTestsCasts.transformIsInstanceOf` comparing with the erased `Box`):
    /// the JVM and the interpreter make the class test; JavaScript makes the test that the value
    /// is not `null` where it may be (`true`), which Scala.js's optimizer proves in its erased
    /// IR (`OptimizerCore`): `Program::js_test`.
    Proven(TestId, bool),
}

impl TypeTest {
    /// The JVM's class of the values a test of a primitive or a builtin takes, which a failed
    /// cast names (`java.lang.Integer` for `Int`).
    pub fn boxed_class_name(self) -> &'static str {
        match self {
            TypeTest::Number => "java.lang.Double",
            TypeTest::Int => "java.lang.Integer",
            TypeTest::Long => "java.lang.Long",
            TypeTest::Byte => "java.lang.Byte",
            TypeTest::Short => "java.lang.Short",
            TypeTest::Float => "java.lang.Float",
            TypeTest::Str => "java.lang.String",
            TypeTest::Char => "java.lang.Character",
            TypeTest::Bool => "java.lang.Boolean",
            TypeTest::Unit => "scala.runtime.BoxedUnit",
            TypeTest::Array => "[Ljava.lang.Object;",
            TypeTest::Null => "scala.runtime.Null$",
            _ => "java.lang.Object",
        }
    }
}

/// A quote `'{ ... }` or `'[T]`: the typed body with a hole local where each splice stands,
/// the expressions whose `Expr` values fill the holes, and the type parameters free in the body
/// with the expressions whose `Type` values they take. The quote appears in the IR as the
/// template `$quote` applied to its index and those expressions (`src/typer/quoted.rs`).
#[derive(Clone)]
pub struct TQuote {
    pub body: Option<TExprId>,
    /// The type of the body, or the quoted type.
    pub ty: TypeId,
    pub holes: Vec<(SymId, TExprId)>,
    pub types: Vec<(TParamId, TExprId)>,
    /// The locals the body defines, which every run of the quote renames afresh.
    pub binders: Vec<SymId>,
    /// The blocks of the body that declare classes, each with them (`Program::block_classes`):
    /// a run's copy of a block declares them at the run's site (`Copier::declared`).
    pub blocks: Vec<(TExprId, Vec<ClassId>)>,
    /// The `Quotes` the given search found for the quote, which scalac's pickle applies it to
    /// (`'{ e }.apply(q)`, `Type.of[T](q)`); kept for the TASTy writer, which alone reads it.
    pub quotes: Option<TExprId>,
}

/// A quote pattern `case '{ ... }` or `case '[T]`: the typed body with a hole local for each
/// `$x`, in the order the match binds them, and the type variables the pattern binds after them.
#[derive(Clone)]
pub struct TQuotePat {
    pub body: Option<TExprId>,
    pub ty: TypeId,
    pub holes: Vec<SymId>,
    pub type_params: Vec<TParamId>,
    /// How many of `type_params`, the first, the pattern declares (`type t <: AnyVal`).
    pub declared: u32,
    /// Which of `type_params` take their upper bound once the match is done, dotty's `@fromAbove`
    /// (`QuotesAndSplices.typedQuotePattern` gives it a variable the pattern binds in a
    /// contravariant position): the others take their lower bound.
    pub from_above: Vec<bool>,
    /// The type parameters of the enclosing definitions that the pattern mentions, with the
    /// expressions whose `Type` values they take when the match runs.
    pub types: Vec<(TParamId, TExprId)>,
    /// The `Quotes` the given search found for the pattern, the `QUOTEPATTERN`'s operand: found
    /// in a build that captures the bodies alone, which the TASTy writer reads.
    pub quotes: Option<TExprId>,
}

/// An inline method's body typed once, at its definition: the contract between the definition
/// check that makes it, the expansion by substitution and the TASTy writer that read it.
/// The parameters stand as themselves at their declared types and `this` is
/// the enclosing class's; what scalac's typer leaves for an expansion is kept, not done.
#[derive(Clone)]
pub struct InlineDefinition {
    pub state: DefinitionState,
    /// The typed body: none while it is typed and for a body held back.
    pub body: Option<TExprId>,
    /// The type the body has at the definition: the declared result type, or the one inferred.
    pub ty: TypeId,
    /// The parameters in clause order, which the body names as `Local` nodes and an expansion
    /// replaces.
    pub params: Vec<SymId>,
    /// The defaults typed at the definition, parallel to `params`.
    pub defaults: Vec<Option<TExprId>>,
    /// What the body defines (lambda parameters, vals, local defs and their parameters, pattern
    /// binders), each of which an expansion renames afresh.
    pub binders: Vec<SymId>,
    /// The type variables the body's type patterns bind (`case _: List[t]`), each a type
    /// parameter of the body that an expansion makes afresh, as it makes a local method's.
    pub pattern_tparams: Vec<TParamId>,
    /// The `inline if` and `inline match` of the body, as the `If` and `Match` nodes they are
    /// typed to with every branch checked, which an expansion reduces.
    pub reducible: Vec<TExprId>,
    /// Where each of `reducible` was written, for what an expansion that cannot reduce it says.
    pub reducible_sources: Vec<ReducibleSource>,
    /// The calls an expansion makes: of the inline methods the body calls and of the members
    /// of `scala.compiletime`, each a call node with its record among the deferred calls (the
    /// callee, the type arguments, the owner's type arguments, the prefix and the result type).
    pub deferred: Vec<TExprId>,
    /// The type arguments of the body's calls of methods with type parameters, which the call
    /// nodes leave out.
    pub type_args: Vec<(TExprId, Vec<TypeId>)>,
    /// The body's other type trees the nodes leave out, each with the node it stands on: an
    /// ascription's type, a `new`'s class type, the type of an `isInstanceOf` or an
    /// `asInstanceOf`, and, on its block, a local type alias's right-hand side and the paths
    /// a type annotation's arguments name (`Int @Ann(m)`). An expansion's copies take them in
    /// its terms, which `Inliner.dropUnusedDefs`' count reads with the expansion's tree.
    pub type_trees: Vec<(TExprId, Vec<TypeId>)>,
    /// The top-level splices, `TExpr::Splice` nodes.
    pub splices: Vec<TExprId>,
    /// What an expansion takes from its call site (its leaves, `Program::leaf_bits`): the
    /// references to the parameters and to `this`, and the deferred calls of the intrinsics
    /// whose value is the site's.
    pub leaves: Vec<TExprId>,
    /// The type tests of a type that names a type parameter of the method, which an expansion
    /// takes from its type arguments (`Program::leaf_tests`).
    pub leaf_tests: Vec<TestId>,
    /// What typing the body reported, at the definition's positions.
    pub diagnostics: Vec<crate::source::Diagnostic>,
    /// The nodes of the body scalac types at a type that is no literal type, whatever constant
    /// they come to be (`Program::widened_bits`: `(i: Int)`), which an expansion's copies keep
    /// marked: a tuple's index of one is read at run time.
    pub widened: Vec<TExprId>,
    /// The body's plain inline calls `Program::opaque_bits` marks, which its copies keep.
    pub opaque: Vec<TExprId>,
    /// The body's arrays spread into a varargs `Program::spread_bits` marks, which its copies keep.
    pub spread: Vec<TExprId>,
    /// The body's ascriptions `Program::ascribed_bits` marks, which its copies keep.
    pub ascribed: Vec<TExprId>,
    /// The body's block statements `Program::removed_bits` marks, which its copies keep where the copy is pure
    /// by itself (`Worker::removed_copy`).
    pub removed: Vec<TExprId>,
    /// The body's nodes the inline `this` decision has a note for (`InlineState::this_written`), with whether the type
    /// as written is a `this.type`: a `this`, or a cast of one, that a cast, an ascription or the method's result type
    /// re-typed, and a root that is a deferred inline call; its copies keep them, which the decision reads in an
    /// expansion.
    pub written_this: Vec<(TExprId, bool)>,
    /// The messages of the body's checks of a tuple's index `Program::tuple_index_bits` marks,
    /// which its copies keep.
    pub tuple_index: Vec<TExprId>,
    /// The body's calls of `erasedValue`, scala-library's a plain method: each copy an expansion
    /// makes is an erased value as the call is (`Erasure.checkNotErased`), which the unit's end
    /// reports where the trees keep it (`Worker::erased_values`).
    pub erased: Vec<TExprId>,
    /// The temporaries the typing hoisted an operand into (`Worker::hoist`), which an expansion
    /// does without where the operand or the call's receiver becomes stable.
    pub hoisted: Vec<SymId>,
    /// The temporaries the typing bound an argument of a right-associative operator's application into
    /// (`Worker::lift_impure`, the reference's `typedInfixOp`). Such a binding stays whatever its initializer
    /// expands to; an expansion does without it only where the call's receiver has become a pure path, beside
    /// which it cannot be told from its absence.
    pub lifted_args: Vec<SymId>,
    /// The imports of the body's blocks, each with the block and the statement it stands before
    /// (the block's result where it stands after the last), which an expansion's searches see
    /// from there to the block's end as they saw them at the definition.
    pub imports: Vec<BlockImport>,
    /// The static type of every node of the body and of its defaults, the record's own in every
    /// build (a build that keeps no types keeps them aside for it, `Program::keep_types_aside`),
    /// which the expansion reads.
    pub node_types: FxMap<TExprId, TypeId>,
    /// The names the body's blocks bind to a value's member (`import v.{m as a}`), which no
    /// import record holds: each with its block, the statement it stands before, the local the
    /// definition bound (a given where the member is one) and the selection it stands for, a
    /// tree of the record typed with the body (`StoredAlias`).
    pub aliases: Vec<StoredAlias>,
    /// The classes the body makes (an anonymous class, a lambda's class of a trait with one
    /// abstract method), typed with it and kept here, out of the program's classes and of the
    /// names the output gives: each expansion copies the one it makes, named then
    /// (`Worker::anon_name`).
    pub classes: Vec<TClass>,
    /// The blocks of the body that declare classes, each with them (`Program::block_classes`):
    /// an expansion's copy of a block declares their copies, made or not (`Copier::declared`).
    pub blocks: Vec<(TExprId, Vec<ClassId>)>,
    /// The body's `inline val`s whose value an inline call kept for the expansion gives
    /// (`inline val size = constValue[Tuple.Size[E]]`), which each expansion's uses take.
    pub inline_vals: Vec<SymId>,
    /// The body's vals whose type the definition inferred from their initialiser, which an
    /// expansion infers again from the walked one, as the retype path types them.
    pub inferred_vals: Vec<SymId>,
    /// A converted body's vals whose pickle wrote their type as inferred (a type, no type tree),
    /// which an expansion keeps as typed: an expansion's count reads the type of each, as of
    /// `inferred_vals`, as dotty's inferred `TypeTree`, whose match types the expansion's terms
    /// reduce (`export_type_trees`).
    pub pickle_inferred_vals: Vec<SymId>,
    /// The body's calls of `type_args` whose type arguments the source wrote: those of any other
    /// were inferred, which an expansion's count reads as dotty's inferred `TypeTree`
    /// (`export_type_trees`).
    pub written_type_args: Vec<TExprId>,
    /// The first form of the checked body the expansion by substitution does not take, found
    /// once when the record is made (`Worker::walk_lack`).
    pub walk_lack: Option<WalkLack>,
}

/// A form of a checked body the expansion by substitution does not take, which leaves its calls
/// to the retype path (`substitution::Fallback`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WalkLack {
    /// `summonFrom` kept as a call (in a quote of the body).
    SummonFrom,
    /// Another form, named.
    Lacks(&'static str),
}

/// A name a block of a stored inline body binds to a value's member (`import v.{m as a}`): the
/// block, the statement it stands before, the local the definition bound and the selection it
/// stands for, which an expansion copies and walks, binding a fresh local to it from there to the
/// block's end, a given where the local is one.
#[derive(Clone, Copy)]
pub struct StoredAlias {
    pub block: TExprId,
    pub at: u32,
    pub local: SymId,
    pub tree: TExprId,
}

/// Where an `inline if` or `inline match` of a stored body was written, in the definition's
/// file: the whole, and the condition, or the scrutinee and each case's pattern and guard.
#[derive(Clone, Debug)]
pub enum ReducibleSource {
    If { whole: Span, cond: Span },
    Match { whole: Span, scrut: Span, cases: Vec<(Span, Option<Span>)> },
    /// A `summonFrom` whose cases the definition check typed: the call and each case's pattern
    /// and guard.
    SummonFrom { whole: Span, cases: Vec<(Span, Option<Span>)> },
}

/// An import of a block of a stored inline body: the block, the statement it stands before, and
/// the import as the definition resolved it.
#[derive(Clone, Copy)]
pub struct BlockImport {
    pub block: TExprId,
    pub at: u32,
    pub import: crate::typer::ResolvedImport,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DefinitionState {
    /// Being typed, which a demand for the body meets in a cycle.
    InProgress,
    /// Typed without an error.
    Checked,
    /// Typed with errors, which `diagnostics` holds.
    Failed,
    /// Not typed: the body has a form the definition check does not type yet.
    Held(HeldForm),
}

/// A form of body the definition check leaves to the expansion for now.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum HeldForm {
    /// A class, trait or object defined in the body, or an anonymous class, which typing the
    /// body would add to the program's classes.
    LocalClass,
    /// A lambda where a trait with one abstract method is expected, which becomes a class.
    SamLambda,
    /// `summonFrom` of a case the definition does not type (a pattern other than `x: T`, `_:
    /// T` and `_`, a guard), or in a quote.
    SummonFrom,
}

impl HeldForm {
    pub fn name(self) -> &'static str {
        match self {
            HeldForm::LocalClass => "local class",
            HeldForm::SamLambda => "SAM lambda",
            HeldForm::SummonFrom => "summonFrom",
        }
    }
}

#[derive(Clone)]
pub struct TFun {
    pub sym: SymId,
    pub params: Vec<SymId>,
    /// Default value expressions, parallel to `params`.
    pub defaults: Vec<Option<TExprId>>,
    pub body: Option<TExprId>,
    /// Whether the type the body was typed at, before its adaptation to the declared result,
    /// is a constant type or `Nothing` (`CheckUnused.isUnconsuming`'s tests of `rhs.tpe`; a
    /// call of a method of a literal result type has its literal type): read off the type when
    /// the body is committed, where an unused kind is checked.
    pub body_unconsuming: bool,
    /// How many of `params`, from the front, are the proxies of a local trait's member: the
    /// values of the block its body reads, which each class mixing the trait in holds and passes
    /// (dotty's `LambdaLift.addFreeParams`, `Worker::lift_trait_proxies`). Not the method's own:
    /// the signature and the pickle leave them out.
    pub captures: u32,
    /// The member of a local trait (owned by a term or by such a trait, dotty's
    /// `Dependencies.isLocal`) whose proxies every call passes (`LambdaLift.addFreeArgs`): its
    /// lifted method is its one slot, which a class's member of the source signature does not
    /// override, and a call of it passes `captures` arguments before its own.
    pub lifted: bool,
}

/// The call of a parent constructor as the typer hands it over: `TClass::parent_prelude` and
/// `TClass::parent_args`.
#[derive(Clone, Copy)]
pub struct ParentCall {
    pub prelude: ListRef,
    pub args: ListRef,
    /// The secondary constructor the arguments go to, when they resolve to one.
    pub via: Option<SymId>,
}

impl ParentCall {
    pub const NONE: ParentCall = ParentCall { prelude: ListRef::EMPTY, args: ListRef::EMPTY, via: None };
}

#[derive(Clone)]
pub enum TInit {
    Field(SymId, TExprId),
    Stmt(TExprId),
    /// The body of an ancestor trait, run in linearisation order, with what the class passes to
    /// the parameters of the trait. The evidence of a trait that takes nothing else is set with
    /// `Field` instead.
    Parent(ClassId, ParentCall),
}

/// The members a class may take by rule (`TClass::synthesized`), dotty's `caseClassSynthesized`
/// in its order and then an enum value's `ordinal`, each with the parameter it takes: `Any` for
/// the two equalities, `Int` for the two element accessors.
pub mod synthetic {
    use crate::intern::Name;
    use crate::names;

    pub const HASH_CODE: u16 = 1 << 0;
    pub const EQUALS: u16 = 1 << 1;
    pub const TO_STRING: u16 = 1 << 2;
    pub const CAN_EQUAL: u16 = 1 << 3;
    pub const PRODUCT_ARITY: u16 = 1 << 4;
    pub const PRODUCT_PREFIX: u16 = 1 << 5;
    pub const PRODUCT_ELEMENT: u16 = 1 << 6;
    pub const PRODUCT_ELEMENT_NAME: u16 = 1 << 7;
    pub const ORDINAL: u16 = 1 << 8;
    /// With `EQUALS`: the synthesized `equals` ends with `that.canEqual(this)`, the class not
    /// being final or having a `canEqual` of its own (`SyntheticMembers.equalsBody`).
    pub const EQUALS_CALLS_CAN_EQUAL: u16 = 1 << 15;
    /// `HASH_CODE`, `EQUALS` or `TO_STRING` shifted by this: none is synthesized, the existing
    /// definition (`existingDef`) being the superclass's or one it inherits, which the JS
    /// registration of a shape leaves to the prototype chain.
    pub const INHERITED: u32 = 9;

    /// A case class's (`caseSymbols`); a case object's lacks `equals` (`caseModuleSymbols`).
    pub const CASE: u16 = HASH_CODE | EQUALS | TO_STRING | CAN_EQUAL | PRODUCT_ARITY | PRODUCT_PREFIX | PRODUCT_ELEMENT | PRODUCT_ELEMENT_NAME;
    /// An enum value's (`nonJavaEnumValueSymbols`); one of a Java enum takes `productPrefix` alone.
    pub const ENUM_VALUE: u16 = PRODUCT_PREFIX | TO_STRING | ORDINAL | HASH_CODE;
    /// A value class's (`valueSymbols`).
    pub const VALUE: u16 = HASH_CODE | EQUALS;

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Param {
        None,
        Any,
        Int,
    }

    pub const MEMBERS: [(u16, Name, Param); 9] = [
        (HASH_CODE, names::HASH_CODE, Param::None),
        (EQUALS, names::EQUALS, Param::Any),
        (TO_STRING, names::TO_STRING, Param::None),
        (CAN_EQUAL, names::CAN_EQUAL_METHOD, Param::Any),
        (PRODUCT_ARITY, names::PRODUCT_ARITY, Param::None),
        (PRODUCT_PREFIX, names::PRODUCT_PREFIX, Param::None),
        (PRODUCT_ELEMENT, names::PRODUCT_ELEMENT, Param::Int),
        (PRODUCT_ELEMENT_NAME, names::PRODUCT_ELEMENT_NAME, Param::Int),
        (ORDINAL, names::ORDINAL, Param::None),
    ];
}

/// How a `final val` of a constant type is stored (`constant_val`): its initialiser is the
/// literal, or the literal is the type it declares and the initialiser runs for its effects.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConstVal {
    Literal,
    Declared(crate::types::LitId),
}

/// dotty's `SymUtils.isConstExprFinalVal`, the storage `Memoize`, `Mixin` and `Constructors` give a
/// `final val` of a constant type: no field holds it and its getter answers the constant, an
/// initialiser that is no literal running for its effects alone. The constant type is the literal
/// one the val declares (or a pickle's), or the literal an untyped val is initialised with, to
/// which the typer folds its reads (`Worker::constant_step`). A member of a JavaScript type keeps
/// its field (`sjsNeedsField`).
pub fn constant_val(prog: &Program, syms: &crate::symbols::Symbols, types: &crate::types::TypeStore, asts: &crate::ast::Asts, s: SymId, init: TExprId) -> Option<ConstVal> {
    use crate::ast::mods;
    use crate::symbols::{JsKind, Owner, SymKind};
    let info = syms.sym(s);
    if info.kind != SymKind::Val || info.mods & mods::FINAL == 0 || info.mods & (mods::LAZY | mods::GIVEN | mods::INLINE) != 0 {
        return None;
    }
    if !matches!(info.owner, Owner::Class(c) if syms.class(c).js == JsKind::Scala) {
        return None;
    }
    let literal = matches!(prog.expr(init), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_));
    if let Some(crate::types::Type::Lit(l)) = info.sig.as_ref().map(|sig| types.get(sig.ret)) {
        return Some(if literal { ConstVal::Literal } else { ConstVal::Declared(l) });
    }
    let file = info.file.0 as usize;
    let untyped = file < asts.len() && info.def.is_some_and(|d| asts[file].defs.get(d.idx()).is_some_and(|def| matches!(def.kind, crate::ast::DefKind::Val { ty: None, .. })));
    (untyped && literal && !prog.is_widened(init)).then_some(ConstVal::Literal)
}

/// dotty's `SyntheticMembers.chooseHashcode`: whether the case class `c` has fields and none of a
/// primitive class (`isPrimitiveValueClass`), so that its `hashCode` by rule is
/// `MurmurHash3.productHash`, through `productArity` and `productElement`, rather than the mix of
/// its fields.
pub fn hashes_through_elements(syms: &crate::symbols::Symbols, types: &crate::types::TypeStore, c: ClassId) -> bool {
    let fields = case_field_types(syms, c);
    !fields.is_empty() && !fields.into_iter().any(|t| t.is_some_and(|t| is_primitive_value_class(syms, types, t)))
}

/// dotty's `SyntheticMembers.equalsBody`'s `sortedAccessors`: the indices of the case class
/// `c`'s fields (its first parameter list, `Product`'s order) in the order its `equals` by rule
/// compares them, those of a primitive class first (`isPrimitiveValueClass`), each group in
/// declaration order, so that a cheap unequal field decides before an `equals` of a reference.
/// Whether `equals_order` of `c` is another than the fields' order: a reference field comes
/// before a primitive one.
pub fn equals_reorders(syms: &crate::symbols::Symbols, types: &crate::types::TypeStore, c: ClassId) -> bool {
    let info = syms.class(c);
    let Some(cl) = info.ctor.first() else { return false };
    if cl.params.len() < 2 {
        return false;
    }
    let mut reference_seen = false;
    for p in &cl.params {
        let primitive = !p.repeated && syms.sym(p.sym).declared_alias().is_none() && is_primitive_value_class(syms, types, p.ty);
        if primitive && reference_seen {
            return true;
        }
        reference_seen |= !primitive;
    }
    false
}

pub fn equals_order(syms: &crate::symbols::Symbols, types: &crate::types::TypeStore, c: ClassId) -> Vec<usize> {
    // The declared type's symbol (`accessor.info.typeSymbol`), an alias's being no class, where
    // the hash reads the class behind it (`hashes_through_elements`).
    let primitive: Vec<bool> = syms
        .class(c)
        .ctor
        .first()
        .map(|cl| cl.params.iter().map(|p| !p.repeated && syms.sym(p.sym).declared_alias().is_none() && is_primitive_value_class(syms, types, p.ty)).collect())
        .unwrap_or_default();
    let fields = primitive.len();
    let mut order: Vec<usize> = (0..fields).collect();
    // Already in order where no reference comes before a primitive, the usual case.
    if primitive.windows(2).any(|w| !w[0] && w[1]) {
        order.sort_by_key(|&i| !primitive[i]);
    }
    order
}

/// The declared types of the case class's fields, `None` for a repeated one, whose accessor is a
/// `Seq` (`xs: Int*`).
fn case_field_types(syms: &crate::symbols::Symbols, c: ClassId) -> Vec<Option<TypeId>> {
    syms.class(c).ctor.first().map(|cl| cl.params.iter().map(|p| (!p.repeated).then_some(p.ty)).collect()).unwrap_or_default()
}

/// dotty's `isPrimitiveValueClass` of a field's declared type's `typeSymbol`, a literal type's
/// being its class's (`x: 1` is an `Int`).
fn is_primitive_value_class(syms: &crate::symbols::Symbols, types: &crate::types::TypeStore, t: TypeId) -> bool {
    match types.get(t) {
        crate::types::Type::Class(k, _) => {
            let ki = syms.class(k);
            use crate::names::*;
            ki.kind == crate::symbols::ClassKind::Builtin && matches!(ki.name, INT | LONG | DOUBLE | FLOAT | CHAR | BYTE | SHORT | BOOLEAN | UNIT)
        }
        crate::types::Type::Lit(l) => !matches!(types.lit_val(l), crate::types::LitVal::Str(_)),
        _ => false,
    }
}

#[derive(Clone)]
pub struct TClass {
    pub id: ClassId,
    pub ctor_params: Vec<SymId>,
    pub ctor_defaults: Vec<Option<TExprId>>,
    /// How many of `ctor_params`, from the front, are the locals a named local class captures
    /// from the block it is defined in; an anonymous class captures through all of them.
    pub captures: usize,
    /// What an enum case passes to the `$init` of its enum, a class to the constructor of its
    /// superclass, and the place that creates an anonymous class to the superclass of that.
    pub parent_args: Option<ListRef>,
    /// The secondary constructor of the superclass that `parent_args` go to.
    pub parent_via: Option<SymId>,
    /// Statements (in `Program::stmts`) that run before those arguments are passed: the
    /// temporaries of named arguments written out of parameter order.
    pub parent_prelude: ListRef,
    /// The members dotty's `SyntheticMembers.caseAndValueMethods` gives a case class, case object,
    /// enum value or value class (`synthetic`'s bits), with the `ordinal` `Desugar` gives an enum
    /// class case: those of its kind's set that `existingDef` finds no definition of, which the
    /// targets and the products implement by rule. A class the compiler makes without a body (a
    /// tuple) defines none of them.
    pub synthesized: u16,
    /// Whether a case class's companion goes without the synthetic `unapply` (`unapplySeq`),
    /// withdrawn by one it declares or inherits (`Worker::synthetic_unapply_retracted`).
    pub retracted_unapply: bool,
    /// The record's lists that few classes have (`TClassLists`), apart so that a body without
    /// them is no larger: a resident session keeps the bodies of every build since a full one.
    pub lists: Option<Box<TClassLists>>,
    pub init: Vec<TInit>,
    pub methods: Vec<FunId>,
    /// The secondary constructors: the body of each is a block whose one statement is the
    /// self constructor call (`New` or `NewVia`) and whose result runs after it.
    pub ctors: Vec<FunId>,
    /// The targets of the `super` calls made by the traits this class is the first to mix in.
    pub super_accessors: Vec<SuperAccessor>,
    /// Inherited methods that implement an inherited declaration whose name in the JavaScript
    /// output differs from theirs, which happens where the type arguments of a parent make two
    /// alternatives of an overloaded name one method: (declaration, implementation). The class
    /// answers to the name of the declaration by calling the implementation.
    pub bridges: Vec<(SymId, SymId)>,
    /// The deferred givens of the traits this class is the first to extend, each with the final
    /// lazy given that implements it, made without a definition: its initialiser, a `Field` of
    /// `init`, is the search where the class is defined (`Worker::implement_deferred_givens`).
    /// Not a member of the class's table, so that every lookup finds the trait's declaration.
    pub deferred_givens: Vec<(SymId, SymId)>,
}

/// The file `TEQ_PAIRS_DUMP` names, where the record of every class's pairs and what the own pairs' walks record of
/// them are written (tests/pairs-equality.sh, tests/pairs-twin.sh); read once.
pub fn pairs_dump() -> Option<&'static std::ffi::OsStr> {
    static DUMP: std::sync::OnceLock<Option<std::ffi::OsString>> = std::sync::OnceLock::new();
    DUMP.get_or_init(|| std::env::var_os("TEQ_PAIRS_DUMP")).as_deref()
}

/// A control of the pairs' proofs (tests/pairs-controls.sh): `TEQ_PAIRS_CONTROL=<kind>:<class>:<member>[:<other>]`
/// changes the record a class keeps before anything reads it, so the proofs must turn red on that pair. `drop`
/// takes the pair out, `swap` makes another member of the overriding member's owner its target (`other` by name,
/// else another alternative of the member's name), `unbridge` clears its bridged mark. `class` is the class's
/// written name (`*`: every class), `member` the overridden member's name. It acts only under the dump
/// (`pairs_dump`), so no build without the dump is changed.
pub fn pairs_control() -> Option<&'static (String, String, String, Option<String>)> {
    static CONTROL: std::sync::OnceLock<Option<(String, String, String, Option<String>)>> = std::sync::OnceLock::new();
    CONTROL
        .get_or_init(|| {
            pairs_dump()?;
            let spec = std::env::var("TEQ_PAIRS_CONTROL").ok()?;
            let mut parts = spec.split(':').map(str::to_string);
            let (kind, class, member) = (parts.next()?, parts.next()?, parts.next()?);
            Some((kind, class, member, parts.next()))
        })
        .as_ref()
}

/// Appends `text` to the pairs' dump (`pairs_dump`).
pub fn pairs_dump_write(text: &str) {
    use std::io::Write;
    if let Some(path) = pairs_dump() {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(text.as_bytes());
        }
    }
}

/// A pair of a class whose overriding member the class inherits (`Worker::class_pairs`, dotty's
/// `OverridingPairs.Cursor` over the class): `overriding` overrides or implements `overridden` as
/// the class sees both, the two declared by two of its bases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassPair {
    pub overriding: SymId,
    pub overridden: SymId,
    /// The superclass has the pair as its own (`Bridges.BridgesCursor`, whose one parent is the
    /// superclass): its bridge is the superclass's.
    pub bridged: bool,
}

/// The lists of a class body that most classes leave empty (`TClass::lists`).
#[derive(Clone, Default)]
pub struct TClassLists {
    /// The class's pairs whose overriding member it inherits (`ClassPair`), which the back ends
    /// read where the class is: its JavaScript bridges, its JVM bridges, the interpreter's
    /// dispatch on its instances.
    pub inherited_pairs: Vec<ClassPair>,
    /// Where in `TClass::bridges` the bridges the class's inherited pairs made begin (`bridges_of`): the walk's come
    /// first. The interpreter answers those pairs by their symbols, not by these JavaScript names.
    pub pair_bridges_from: u32,
    /// The indices of a case class's fields in the order its synthesized `equals` compares them
    /// (`equals_order`), where that is not their declaration order; empty otherwise.
    pub equals_order: Vec<u16>,
    /// For each member of the kind's set the class does not get by rule, the definition dotty's
    /// `existingDef` takes in its place (`synthetic`'s bit, the member), which a lookup of the name
    /// finds before the bases' other definitions.
    pub existing_defs: Vec<(u16, SymId)>,
    /// Of a companion that is its class's mirror (`Worker::mirror_parent`), the members
    /// `SyntheticMembers.addMirrorSupport` gives it, after its body (`newBody :+`), which
    /// JavaScript and the interpreter run: `fromProduct`, `ordinal`. The JVM backend and the
    /// pickle write them by rule; no table of the class holds them, nor `TClass::methods`.
    pub mirror_members: Vec<FunId>,
    /// The constant vals of the class's body (`constant_val`) with their constants as literals,
    /// which a getter answers whatever the constructor has run (`Memoize`); a right side that is
    /// no literal stays in `init`, the constructor's work.
    pub constants: Vec<(SymId, TExprId)>,
    /// The outer accessor of a case class nested in a class that its synthesized `equals`
    /// compares by identity after the class test, before the fields and `canEqual`: dotty's
    /// `equalsBody` tests `that` against `C`'s `this`-prefixed type, to which `PatternMatcher`
    /// adds the outer test (`addOuterTest`, `x$0.$outer eq this.$outer`).
    pub eq_outer: Option<SymId>,
}

impl TClass {
    pub fn eq_outer(&self) -> Option<SymId> {
        self.lists.as_deref().and_then(|l| l.eq_outer)
    }

    pub fn equals_order(&self) -> &[u16] {
        self.lists.as_deref().map_or(&[], |l| &l.equals_order)
    }

    pub fn existing_defs(&self) -> &[(u16, SymId)] {
        self.lists.as_deref().map_or(&[], |l| &l.existing_defs)
    }

    pub fn inherited_pairs(&self) -> &[ClassPair] {
        self.lists.as_deref().map_or(&[], |l| &l.inherited_pairs)
    }

    /// The bridges the class's inherited pairs made (`TClassLists::pair_bridges_from`), none where it has no pair.
    pub fn is_pair_bridge(&self, index: usize) -> bool {
        self.lists.as_deref().is_some_and(|l| !l.inherited_pairs.is_empty() && index >= l.pair_bridges_from as usize)
    }

    pub fn constants(&self) -> &[(SymId, TExprId)] {
        self.lists.as_deref().map_or(&[], |l| &l.constants)
    }

    pub fn mirror_members(&self) -> &[FunId] {
        self.lists.as_deref().map_or(&[], |l| &l.mirror_members)
    }

    /// The lists, made where the class has none yet.
    pub fn lists_mut(&mut self) -> &mut TClassLists {
        self.lists.get_or_insert_with(Default::default)
    }
}

/// `@jsImport("module", "name")`; the name `default` is the default export and `*` the namespace.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct JsImport {
    pub module: Name,
    pub name: Name,
}

/// Where the nodes that name a definition of a block from at or before that definition's
/// statement stand in the source. The typed trees keep a node's source only in a build that
/// keeps the types, and never as scalac's tree has it (an identifier's own span, a creation's
/// created type), so the typing notes it for these nodes alone, and the forward-reference
/// check, which runs after the inlining phase, reads it (`typer::forward`).
#[derive(Default)]
pub struct ForwardNotes {
    /// By node.
    pub places: FxMap<TExprId, ForwardPlace>,
    /// By pattern: a constructor pattern's place of the constructor it names; a typed pattern's
    /// place and what its written type names.
    pub patterns: FxMap<TPatId, ForwardPattern>,
    /// The files in whose trees a noted node stands: the units the check walks.
    pub units: Vec<FileId>,
    /// By node, a reference to a protected member that needs an accessor no enclosing class can hold
    /// (`ProtectedAccessors`, a phase of the forward-reference check's group): told by the walk where the node stands
    /// in the final trees, after the inliner and the folds, as scalac checks the references that reach that phase.
    pub accessors: FxMap<TExprId, AccessorNote>,
    /// The units of a walk the typer's error held back (`forward_references`), which a session's next build walks.
    pub owed: Vec<FileId>,
}

/// A protected reference's error, as the typing of the reference decided it (`check_protected_accessor`), and the
/// shape of its node then (`TExpr::reference_shape`): a node an expansion took (an inline call's, whose value its
/// expansion's root now is) has another, and its note is no error.
#[derive(Clone)]
pub struct AccessorNote {
    pub file: FileId,
    pub span: Span,
    pub msg: Box<str>,
    pub shape: (std::mem::Discriminant<TExpr>, u32),
}

/// A pattern's note.
#[derive(Clone, Copy)]
pub struct ForwardPattern {
    pub file: FileId,
    pub span: Span,
    /// A typed pattern's (`x: T`, `_: T`): the type alias of a block its type is written by, or
    /// none where it is written by the class its test's type is of. Absent for a constructor
    /// pattern, which names its class.
    pub typed: Option<Option<crate::ast::DefId>>,
}

#[derive(Clone)]
pub struct ForwardPlace {
    pub file: FileId,
    pub span: Span,
    /// A creation's: the definitions of blocks its created type names that stood at or after
    /// the creation's statement (dotty's `ForwardDepChecks.transformNew` over the type's terms).
    pub parts: Vec<SymId>,
    /// A creation's: the type alias the typer resolved its written type to, where the creation
    /// keeps it (it is not the alias's class applied to the alias's own parameters). The
    /// creation names the alias in its class's place (`tree.tpe.typeSymbol` of dotty's `New`):
    /// an alias of a block by its definition, any other alias no definition of a level.
    pub alias: Option<Option<crate::ast::DefId>>,
}

pub struct Program {
    pub exprs: Arena<TExpr>,
    pub pats: Arena<TPat>,
    pub strings: Arena<String>,
    pub expr_lists: Arena<TExprId>,
    pub pat_lists: Arena<TPatId>,
    pub sym_lists: Arena<SymId>,
    pub stmts: Arena<TStmt>,
    pub cases: Arena<TCase>,
    pub tries: Arena<TTry>,
    pub tests: Arena<TypeTest>,
    /// Tests against a type parameter of a quote, which pass as they stand and are derived
    /// anew where the quote is instantiated with the type filled in.
    pub deferred_tests: FxMap<TestId, TypeId>,
    pub funs: Arena<TFun>,
    pub classes: Arena<TClass>,
    pub top_funs: Registers<FunId>,
    pub top_vals: Registers<(SymId, TExprId)>,
    pub main: Option<SymId>,
    /// The object `main` runs on when the object inherits it from a trait or class.
    pub main_object: Option<ClassId>,
    /// For each alternative of an overloaded name, the inherited methods it overrides: what a
    /// backend that dispatches by name and erased parameters needs to bridge. And for each var
    /// that implements an abstract setter `x_=`, that setter (`typer::setters`).
    pub overrides: crate::arena::Layered<SymId, Vec<SymId>>,
    /// For a method that a class's inherited pair makes implement a method declaring defaults
    /// it does not declare itself (a trait's `f(a: Int)` mixed beside another's
    /// `f(a: Int = 3)`), the members it so implements: the JavaScript function of the method
    /// computes such a default from the getter on `this` (`emit::defaults::js_declarer`). Made
    /// again from the classes' pairs at every build (`Worker::record_class_pairs`).
    pub inherited_defaults: crate::intern::FxMap<SymId, Vec<SymId>>,
    /// The trait whose methods the runtime gives to partial function values.
    pub partial_function: Option<ClassId>,
    /// `scala.reflect.ClassTag`'s companion, whose tags a `@jvmEvidence` member is passed as
    /// `null` outside the JVM (`Program::is_standard_class_tag`).
    pub class_tag_module: Option<ClassId>,
    /// `java.lang.Throwable` and `js.JavaScriptException`, which a catch that wraps refers to.
    pub throwable: Option<ClassId>,
    pub js_exception: Option<ClassId>,
    pub js_imports: Vec<JsImport>,
    /// `@jsExport("name")` definitions with their exported names.
    pub js_exports: Vec<(SymId, Name)>,
    /// Whether the static types of expressions are kept, which the JVM backend reads.
    pub record_types: bool,
    /// Set where `record_types` is for completion's receivers alone (a `--index` session that
    /// keeps no types otherwise): the types are kept, the sources of the expressions are not.
    pub types_only: bool,
    /// The typed form of the bodies that their pickle needs, in the product modes
    /// (`capture.rs`); `None` in every other build.
    pub capture: Option<Box<capture::Capture>>,
    /// Set while a build that keeps no types keeps those of an inline body under the definition
    /// check aside for its record (`Program::keep_types_aside`): `record_types` is set
    /// meanwhile, the types go to `aside_types` and `type_of` does not see them, so that the
    /// body is typed as it is without them.
    pub types_aside: u32,
    pub aside_types: FxMap<TExprId, TypeId>,
    /// The members called through a `@js` or `@jvm` template on a receiver
    /// (`Throwable.getMessage`, `Product.productElement`): a virtual call, which the overrides
    /// in the program's classes have to survive.
    /// Each with the file whose typing made the call (the caller's, for a call an inline
    /// expansion made): a retype of the file takes its calls back.
    pub template_calls: Vec<(SymId, FileId)>,
    /// The files whose typing met a `getClass`, the caller's for one an inline expansion made:
    /// while there is one, every class carries its qualified name.
    pub get_class_units: Vec<FileId>,
    /// A local enum's name in its lookups' messages, dotty's `enumClass.fullName` under the class
    /// or file object whose body defines it (`Worker::enum_full_name`), which the JVM's companion
    /// members write too.
    pub local_enum_names: Vec<(ClassId, String)>,
    /// The def behind each `@js` template call, by the template's string: what the interpreter
    /// runs a builtin for, since the text alone does not name it.
    pub template_syms: crate::arena::Layered<StrRef, SymId>,
    /// While the workers share the program, the `TClass` of each class a worker checked into
    /// its own chunk, published with the bodies and signatures that reach the class: what
    /// another worker's interpreter finds the class's body by. Dropped
    /// at the merge, where every body is placed.
    pub class_bodies: crate::arena::Layered<ClassId, u32>,
    /// Per expression the type the typer gave it, `NO_TYPE` for a node it made on the side.
    pub expr_types: Parallel<TypeId>,
    /// Per expression the source it was typed from, `NO_SPAN` for a node made on the side;
    /// recorded with the types, for the positions a macro reads.
    pub expr_spans: Parallel<(FileId, Span)>,
    /// Per file its `source::file_tag`.
    pub file_tags: crate::arena::FileVec<u64>,
    pub quotes: Arena<TQuote>,
    pub quote_pats: Arena<TQuotePat>,
    /// The inline expansions by the expression each gave, on JavaScript: the JavaScript output
    /// writes the expansions of one method that are one shape as calls of one function
    /// (`emit/outline.rs`).
    pub expansions: FxMap<TExprId, Expansion>,
    /// Per block of the program's code, the named classes it declares, which the IR lifts out of
    /// the block: its own, or in an expansion's copy of a block the copies of its template's
    /// (`TQuote::blocks`, `InlineDefinition::blocks`). A module's products write what a block the
    /// reach walks declares, made or not, as scalac's backend writes the class definitions of
    /// the trees it is given (`reach::Walker::declared`).
    pub block_classes: FxMap<TExprId, Vec<ClassId>>,
    /// The places the forward-reference check tells its errors at (`typer::forward`).
    pub forward: ForwardNotes,
    /// What an inline method's call gave, as bits by expression, on every target: the keys of
    /// `expansions` on JavaScript, which a walk tests at every node. Where a chain of `+` ends
    /// is `chain_end_bits`' to say, which a transparent method's call does not set.
    pub expansion_bits: Bits,
    /// Where a chain of `+` ends, as bits by expression, whatever the node is or comes to be:
    /// what a plain inline method's call gave (a transparent one's carries the chain on), and
    /// the node of an ascription, a cast, a `toString` of a string or another call that the
    /// typer wrote as the node inside it (`Program::chain_head`).
    pub chain_end_bits: Bits,
    /// Which branch the condition of an `if` selects where scalac folds the condition away, by
    /// expression, the `then` branch's bits and the `else` branch's: scalac puts that branch in
    /// the place of the `if`, so a chain of `+` goes on in it.
    pub taken_then_bits: Bits,
    pub taken_else_bits: Bits,
    /// The expressions an ascription types at a type that is no literal type whatever constant
    /// they come to be (`(i: Int)`), as bits by expression, and what stands for one in a copy or
    /// an expansion. A tuple's index of a constant among them, or among `opaque_bits`, is read
    /// at run time (`Worker::widened_constant`).
    pub widened_bits: Bits,
    /// The calls of plain inline methods at a declared type that is no literal type, as bits by
    /// expression: scalac's typer keeps such a call until its inlining phase, so no operation
    /// over it folds (`idx + 0` of an `inline def idx: Int = 2`), where one over an ascription
    /// does (`ConstFold`'s `ConstantTree` looks through `Typed`). The two marks are kept apart: an
    /// ascription of such a call is in both.
    pub opaque_bits: Bits,
    /// The copies of an inline method's by-name argument where the expansion reads its parameter
    /// (`Worker::inline_arg`), as bits by expression: dotty binds such an argument to a proxy def,
    /// so a `return` in it is in no tail position of the method around the call
    /// (`Program::tail_jumps`).
    pub by_name_arg_bits: Bits,
    /// The arrays spread into a varargs as they are written (`f(arr*)`), as bits by expression:
    /// the sequence the typer converts one to is no sequence a program wrote, and a Java
    /// parameter takes the array itself, as scalac passes it.
    pub spread_bits: Bits,
    /// The ascriptions, as bits by expression: an expression the program ascribes a type
    /// (`(e: T)`), the root of a plain inline method's expansion, which dotty ascribes its declared
    /// result (`PrepareInlineable.wrapRHS`), and the operand of a cast that keeps no node (an
    /// upcast, which `TypeTestsCasts.transformAsInstanceOf` makes an ascription, and a cast a
    /// passed test leaves untested). The node keeps no tree of its own, its recorded type is the
    /// ascribed one, which erasure retains (`Erasure.typedTyped`) where the node's own erasure
    /// would be another (a generic result's `Object`): the JVM's qualifier of a member is of the
    /// ascribed class (`Gen::qualifier_site`).
    pub ascribed_bits: Bits,
    /// On the JVM, the nulls an inline expansion wrote from the body it inlined, as bits by expression: those
    /// `Worker::type_null` types while an expansion's body is typed, the copies of a stored body's
    /// (`publish_demanded`, `Worker::inlined_null`) and those a macro's quote writes (`Worker::run_quote`), kept by
    /// every copy that keeps an ascription (an inline argument's, a quote's, a beta-reduced closure's). None is
    /// tested alone (`Gen::null_literal`): the ascriptions and braces of a body's source are in no pickle of teq's,
    /// so a body read from products holds bare nulls where the same module's holds marked ones, and only thus do
    /// the two builds write the same comparison.
    pub inlined_null_bits: Bits,
    /// The nodes a cast's tree stands for (`typer::MARK_CAST`: a primitive's conversion made as a
    /// cast, a mirror's value), as bits by expression: what a macro's view of a body another worker
    /// typed reads (`tree_type`), every worker's marks in one set while forked.
    pub cast_bits: Bits,
    /// The setter calls the typer made of an assignment through an abstract var (`c.x = v`,
    /// `Worker::assignment`), as bits by expression: scalac's tree keeps an assignment, where a
    /// call written `c.x_=(v)` is an application (dotty's `Typer.typedAssign`), which the trees a
    /// macro sees, the pickle and the dependencies tell apart.
    pub setter_assign_bits: Bits,
    /// The statements of blocks that the reference's `PureStats` removes (`transformBlock`: a pure statement
    /// of a block; the pure result of an ascribed block that is a statement), as bits by expression: the
    /// pickle, a quoted tree and every copying walk hold the statement as it was written, which is what the
    /// reference pickles; what runs a block (`Program::runs`, `Program::run_stmts`) leaves it out.
    pub removed_bits: Bits,
    /// The messages of the `compiletime.error` calls a definition's check makes of a literal index
    /// past a tuple's elements (`Worker::deferred_index_error`), as bits by expression, which their
    /// copies keep: what reports such a call's error as `TupleOptimizations`', after `Inlining`.
    pub tuple_index_bits: Bits,
    /// The plain inline calls the typer kept for the inlining phase, as bits by expression,
    /// published with their records: what tells another worker's interpreter that a body it is
    /// about to run holds a call no expansion replaced yet (`Worker::prepare_executable`).
    pub kept_bits: Bits,
    /// What an expansion took from its call site rather than from the body, as bits by
    /// expression: the arguments bound to the parameters, the receiver, the constants of
    /// `constValue` and the givens found. Where such an expression does not name a local of the
    /// expansion, it is an argument of the shared function.
    pub leaf_bits: Bits,
    /// The type tests of an expansion whose type came from a type argument of the call.
    pub leaf_tests: FxMap<TestId, ()>,
    /// The operands dotty's `evalOnce` leaves in place, idempotent by `exprPurity`
    /// (`Worker::mark_in_place_operands`): a generic value class's receiver, which is otherwise
    /// evaluated before the companion (`emit::receiver_first`), and a value of a value class that
    /// erasure may open after a null test, which is otherwise held in a temporary for it.
    pub in_place_operands: FxMap<TExprId, ()>,
    /// The type tests and patterns of the inline bodies stored at their definitions
    /// (`InlineDefinition`), which no output holds: what numbers the classes of the output
    /// from every test of the program leaves them out.
    pub stored_tests: FxMap<TestId, ()>,
    pub stored_pats: FxMap<TPatId, ()>,
    /// How many times a retype took a file's definitions out of `classes`, `top_funs` and
    /// `top_vals` (`Worker::forget_typing`, the one place that shortens them): what stands
    /// behind them moves down and the file's are typed onto the end, so a position or a count
    /// over these lists from before is stale while the lengths are what they were.
    pub shifts: u32,
    /// `shifts` where the build under way began (a session's retype, `retype_files`): the shifts
    /// after it are the inlining phase's retypes of the build's own suspended files.
    pub build_start: u32,
    /// The runs the inlining phase made after the build's first (`Worker::inlining_phase`'s rounds):
    /// a macro's first run in a later one makes the program's objects anew, and from the third run
    /// on the classpath's too, as dotty's third and each later run take a macro class loader of
    /// their own over the classpath and the earlier runs' output (`Driver.finish`,
    /// `MacroClassLoader.init`; `interp::loader_generation`).
    pub runs: u32,
    /// Whether a function is a local trait's lifted member (`TFun::lifted`), shared with the
    /// workers' programs: the interpreter looks for one at a call only then.
    pub lifted_funs: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Whether a trait a class of a block owns was entered, shared with the workers' programs: a
    /// read of a block's value across one is looked for only then (`Worker::held_holder`).
    pub block_class_traits: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// An inline method's expansion: the method expanded, and the call the expansion replaced.
#[derive(Clone, Copy, Debug)]
pub struct Expansion {
    pub callee: SymId,
    /// The call the expansion stands for, as dotty's `Inlined(call, ..)` keeps it: the call's own
    /// typed arguments and its receiver in the order `PostTyper` visits the call (the arguments,
    /// then the function's qualifier). Empty where the call was not recorded: one is where an
    /// argument or the receiver holds a trait's `super` call (`Worker::replaced_call`), for the
    /// order of the trait's super accessors, which is the call's and not the expansion's.
    pub replaced: ListRef,
}

/// Who answers a name a bridge of a class would stand under (`TClass::bridges`).
impl Program {
    /// Whether the bridge that answers `declared`'s output name with `implementation`, a method the
    /// class `tc` itself declares, would stand where another method of the class goes by that
    /// name, for a reader that goes by output names: one the class declares with a body, or one
    /// with a body that it inherits and `implementation` does not override. That method answers the name:
    /// the JavaScript emitter does not write the bridge (`emit_bridges`), the reach takes nothing
    /// through it (`mark_bridge`) and the interpreter does not take it (`own_member`); the JVM,
    /// whose methods go by their descriptors, has its own skip (`emit_inherited_bridges`). A bridge
    /// to an inherited implementation, or to a value, stands.
    pub fn bridge_name_held(&self, syms: &Symbols, is_abstract: &dyn Fn(SymId) -> bool, tc: &TClass, declared: SymId, implementation: SymId) -> bool {
        let implemented = syms.sym(implementation);
        implemented.owner == Owner::Class(tc.id) && implemented.kind == SymKind::Def && self.name_held_beside(syms, is_abstract, tc, declared, implementation)
    }

    /// Whether a method of the class `tc` other than `implementation` goes by `declared`'s output
    /// name: an alternative of the declaration's own name, one the class declares with a body, or
    /// one with a body that it inherits, that is not the declaration and that `implementation`
    /// does not override. A method of another name that carries the same output name (a
    /// `@targetName`) is no such holder: the two were not named apart from one another.
    pub fn name_held_beside(&self, syms: &Symbols, is_abstract: &dyn Fn(SymId) -> bool, tc: &TClass, declared: SymId, implementation: SymId) -> bool {
        let name = syms.dispatch_name(declared);
        let source = syms.sym(declared).name;
        let own = tc.methods.iter().any(|&m| {
            let fun = &self.funs[m.idx()];
            fun.sym != implementation && fun.body.is_some() && syms.sym(fun.sym).name == source && syms.dispatch_name(fun.sym) == name
        });
        if own {
            return true;
        }
        let overridden = self.overrides.get(&implementation);
        let parameters = |s: SymId| syms.sym(s).sig.as_ref().map(|sig| sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect::<Vec<TypeId>>());
        let declared_parameters = parameters(declared);
        for &(b, _) in syms.class(tc.id).base_types.iter().skip(1) {
            let Some(&entry) = syms.class(b).members.get(&source) else { continue };
            let alternatives = match syms.alternatives(entry) {
                Some(alternatives) => alternatives,
                None => std::slice::from_ref(&entry),
            };
            for &x in alternatives {
                let info = syms.sym(x);
                if x == declared || info.owner != Owner::Class(b) || info.kind != SymKind::Def || info.mods & mods::PRIVATE != 0 {
                    continue;
                }
                if is_abstract(x) || syms.dispatch_name(x) != name {
                    continue;
                }
                // What `implementation` overrides is the declaration's own member under that name.
                if overridden.is_some_and(|o| o.contains(&x)) || parameters(x) == declared_parameters {
                    continue;
                }
                return true;
            }
        }
        false
    }
}

impl Program {
    /// A tag as scalac's `Synthesizer` makes it, and as a program or a library body may write it:
    /// a val of `ClassTag`'s companion (`ClassTag.Int`), `ClassTag.apply(classOf[C])`, or such a
    /// tag's `wrap`. Pure, and its `newArray` is the standard allocation, so that a `@jvmEvidence`
    /// member of the std is passed `null` for it outside the JVM, which leaves its array as it is
    /// (`std/buffer.scala`'s `evidenced`).
    pub fn is_standard_class_tag(&self, te: TExprId, syms: &crate::symbols::Symbols, interner: &crate::intern::Interner) -> bool {
        self.standard_class_tag_where(te, syms, interner, &|_, _| None)
    }

    /// `is_standard_class_tag` of a tree in which a node may stand for several: `stands(e, test)`
    /// is `None` for a node that is itself, and for one that stands for others whether `test`
    /// holds of each of them. The emitter's, while it writes the body of an outlined function
    /// from one expansion's tree: a parameter of the function stands for the argument of every
    /// expansion that shares it, wherever on the tag's path it is met (the tag itself, the tag a
    /// `wrap` is taken of, the class a tag is made from, the companion).
    pub fn standard_class_tag_where(
        &self,
        te: TExprId,
        syms: &crate::symbols::Symbols,
        interner: &crate::intern::Interner,
        stands: &dyn Fn(TExprId, &dyn Fn(TExprId) -> bool) -> Option<bool>,
    ) -> bool {
        if let Some(each) = stands(te, &|x| self.is_standard_class_tag(x, syms, interner)) {
            return each;
        }
        let Some(m) = self.class_tag_module else { return false };
        let is = |e: TExprId, test: &dyn Fn(TExprId) -> bool| stands(e, test).unwrap_or_else(|| test(e));
        let companion = |r: TExprId| is(r, &|x| matches!(self.expr(x), TExpr::Module(c) if c == m));
        match self.expr(te) {
            TExpr::Field(r, _) => companion(r),
            TExpr::CallMethod(r, s, args) => {
                let name = interner.get(syms.sym(s).name);
                if companion(r) {
                    name == "apply" && matches!(self.expr_list(args), [a] if is(*a, &|x| matches!(self.expr(x), TExpr::ClassOf(..))))
                } else {
                    name == "wrap" && args.is_empty() && self.standard_class_tag_where(r, syms, interner, stands)
                }
            }
            _ => false,
        }
    }

    /// The arguments of `call` that are evidence of a `@jvmEvidence` member, a bit per position in
    /// the call's argument list, whose standard tags the JavaScript output passes as `null`
    /// (`is_standard_class_tag`): the parameters of its implicit clauses, the tag scala-library's
    /// member is passed as evidence, and no ordinary argument (`appended(ClassTag.Int)` keeps its
    /// element). Only of a member nothing outside the std overrides, which reads the `null` as a
    /// standard tag: an extension, a top-level definition, an object's or a final class's member;
    /// a class of a library that overrides `toArray` (zio's `Chunk`) is passed the tag itself.
    pub fn evidence_args(&self, call: TExprId, syms: &crate::symbols::Symbols) -> u64 {
        let (s, n) = match self.expr(call) {
            TExpr::CallMethod(_, s, args) | TExpr::CallStatic(s, args) => (s, args.len as usize),
            TExpr::Js(t, args) => match self.template_syms.get(&t) {
                Some(&s) => (s, args.len as usize),
                None => return 0,
            },
            _ => return 0,
        };
        self.evidence_of(s, n, syms)
    }

    /// Whether the argument `a` at the position `k` of a call is a standard tag in one of the
    /// call's evidence positions (`evidence`: `evidence_args`, or `std_evidence_of`): what the
    /// JavaScript output does not evaluate at the call. An ordinary call writes `null` for it,
    /// a text's placeholder too, and reachability does not visit it.
    pub fn is_dropped_tag(&self, evidence: u64, k: usize, a: TExprId, syms: &crate::symbols::Symbols, interner: &crate::intern::Interner) -> bool {
        k < 64 && evidence & 1 << k != 0 && self.is_standard_class_tag(a, syms, interner)
    }

    /// `evidence_of` for a member the std itself defines, and nothing for any other: the
    /// positions whose standard tag a text of the member does not bind. A text that names the
    /// tag writes `null` there and one that leaves it out writes nothing, where an argument a
    /// program's own text leaves out or names under a condition is evaluated ahead of the text.
    pub fn std_evidence_of(&self, s: SymId, n: usize, syms: &crate::symbols::Symbols) -> u64 {
        if syms.sym(s).std_evidence {
            self.evidence_of(s, n, syms)
        } else {
            0
        }
    }

    /// `evidence_args` of a call of the member `s` with `n` arguments, a template's receiver
    /// among them.
    pub fn evidence_of(&self, s: SymId, n: usize, syms: &crate::symbols::Symbols) -> u64 {
        let info = syms.sym(s);
        let overridable = !info.is_extension
            && match info.owner {
                crate::symbols::Owner::Class(c) => {
                    let class = syms.class(c);
                    class.kind != crate::symbols::ClassKind::Object && class.mods & crate::ast::mods::FINAL == 0
                }
                _ => false,
            };
        if !info.jvm_evidence || overridable {
            return 0;
        }
        let Some(sig) = info.info.sig.as_deref() else { return 0 };
        let params = sig.clauses.iter().map(|c| c.params.len()).sum::<usize>();
        // A template's arguments begin with its receiver where the member has one.
        let Some(offset) = n.checked_sub(params) else { return 0 };
        let mut mask = 0u64;
        let mut k = offset;
        for clause in &sig.clauses {
            for _ in &clause.params {
                if (clause.is_using || clause.is_implicit) && k < 64 {
                    mask |= 1 << k;
                }
                k += 1;
            }
        }
        mask
    }

    /// Whether a function is a local trait's lifted member (`lifted_funs`).
    #[inline]
    pub fn has_lifted_funs(&self) -> bool {
        self.lifted_funs.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Records that a function is a local trait's lifted member.
    pub fn note_lifted_fun(&self) {
        self.lifted_funs.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether a trait a class of a block owns was entered (`block_class_traits`).
    #[inline]
    pub fn has_block_class_traits(&self) -> bool {
        self.block_class_traits.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Records that a trait a class of a block owns was entered.
    pub fn note_block_class_trait(&self) {
        self.block_class_traits.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Another worker's program over the same shared regions: its records are its own, the
    /// registers (the top-level definitions, the expansions, ...) merged after the bodies.
    pub fn attach(&self, worker: usize) -> Program {
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut p = self.attached(worker);
        #[cfg(debug_assertions)]
        p.follow_escapes();
        p
    }

    /// Makes the records so far the shared regions, which the body phase's workers read by
    /// reference (`attach`), this program worker 0's over them.
    pub fn fork(&mut self) {
        self.exprs.fork();
        self.pats.fork();
        self.strings.fork();
        self.expr_lists.fork();
        self.pat_lists.fork();
        self.sym_lists.fork();
        self.stmts.fork();
        self.cases.fork();
        self.tries.fork();
        self.tests.fork();
        self.funs.fork();
        self.classes.fork();
        self.quotes.fork();
        self.quote_pats.fork();
        self.expr_types.fork();
        self.expr_spans.fork();
        self.expansion_bits.fork();
        self.leaf_bits.fork();
        self.chain_end_bits.fork();
        self.taken_then_bits.fork();
        self.taken_else_bits.fork();
        self.widened_bits.fork();
        self.opaque_bits.fork();
        self.by_name_arg_bits.fork();
        self.spread_bits.fork();
        self.ascribed_bits.fork();
        self.inlined_null_bits.fork();
        self.cast_bits.fork();
        self.setter_assign_bits.fork();
        self.removed_bits.fork();
        self.tuple_index_bits.fork();
        self.kept_bits.fork();
        self.file_tags.fork();
        #[cfg(debug_assertions)]
        self.follow_escapes();
    }

    /// A peer's entry of an expression in a table parallel to the expressions (its type, its
    /// source, its marks) is read as its expression is, once a publication sealed it, marking it
    /// read; the owner's write of the entry is refused after (`Parallel::follow_escapes`,
    /// `Bits::follow_escapes`). A peer reads a record of the program only once sealed.
    #[cfg(debug_assertions)]
    pub fn follow_escapes(&mut self) {
        if let Some(marks) = self.exprs.escape_marks() {
            let own_base = self.exprs.own_base();
            self.expr_types.follow_escapes(marks.clone(), own_base);
            self.expr_spans.follow_escapes(marks.clone(), own_base);
            for bits in [&mut self.expansion_bits, &mut self.leaf_bits, &mut self.chain_end_bits, &mut self.taken_then_bits, &mut self.taken_else_bits, &mut self.widened_bits, &mut self.opaque_bits, &mut self.by_name_arg_bits, &mut self.spread_bits, &mut self.ascribed_bits, &mut self.inlined_null_bits, &mut self.cast_bits, &mut self.setter_assign_bits, &mut self.removed_bits, &mut self.kept_bits, &mut self.tuple_index_bits] {
                bits.follow_escapes(marks.clone(), own_base);
            }
        }
        self.exprs.peers_read_sealed();
        self.pats.peers_read_sealed();
        self.strings.peers_read_sealed();
        self.expr_lists.peers_read_sealed();
        self.pat_lists.peers_read_sealed();
        self.sym_lists.peers_read_sealed();
        self.stmts.peers_read_sealed();
        self.cases.peers_read_sealed();
        self.tries.peers_read_sealed();
        self.tests.peers_read_sealed();
        self.funs.peers_read_sealed();
        self.classes.peers_read_sealed();
        self.quotes.peers_read_sealed();
        self.quote_pats.peers_read_sealed();
    }

    fn attached(&self, worker: usize) -> Program {
        Program {
            exprs: Arena::attach(self.exprs.shared_arc(), worker),
            pats: Arena::attach(self.pats.shared_arc(), worker),
            strings: Arena::attach(self.strings.shared_arc(), worker),
            expr_lists: Arena::attach(self.expr_lists.shared_arc(), worker),
            pat_lists: Arena::attach(self.pat_lists.shared_arc(), worker),
            sym_lists: Arena::attach(self.sym_lists.shared_arc(), worker),
            stmts: Arena::attach(self.stmts.shared_arc(), worker),
            cases: Arena::attach(self.cases.shared_arc(), worker),
            tries: Arena::attach(self.tries.shared_arc(), worker),
            tests: Arena::attach(self.tests.shared_arc(), worker),
            deferred_tests: FxMap::default(),
            funs: Arena::attach(self.funs.shared_arc(), worker),
            classes: Arena::attach(self.classes.shared_arc(), worker),
            top_funs: self.top_funs.attach(),
            top_vals: self.top_vals.attach(),
            main: None,
            main_object: None,
            overrides: self.overrides.attach(),
            inherited_defaults: self.inherited_defaults.clone(),
            partial_function: self.partial_function,
            class_tag_module: self.class_tag_module,
            throwable: self.throwable,
            js_exception: self.js_exception,
            js_imports: Vec::new(),
            js_exports: Vec::new(),
            record_types: self.record_types,
            types_only: self.types_only,
            capture: self.capture.as_ref().map(|c| Box::new(c.attached())),
            types_aside: 0,
            aside_types: FxMap::default(),
            template_calls: Vec::new(),
            get_class_units: Vec::new(),
            local_enum_names: Vec::new(),
            shifts: 0,
            build_start: 0,
            runs: 0,
            lifted_funs: self.lifted_funs.clone(),
            block_class_traits: self.block_class_traits.clone(),
            template_syms: self.template_syms.attach(),
            class_bodies: self.class_bodies.attach(),
            expr_types: Parallel::attach(self.expr_types.shared_arc(), NO_TYPE, worker),
            expr_spans: Parallel::attach(self.expr_spans.shared_arc(), NO_SPAN, worker),
            file_tags: self.file_tags.attach(worker),
            quotes: Arena::attach(self.quotes.shared_arc(), worker),
            quote_pats: Arena::attach(self.quote_pats.shared_arc(), worker),
            expansions: FxMap::default(),
            block_classes: FxMap::default(),
            // A worker of the inlining phase copies and expands the notes' nodes: it reads the protected references'.
            forward: ForwardNotes { accessors: self.forward.accessors.clone(), ..ForwardNotes::default() },
            expansion_bits: Bits::attach(self.expansion_bits.shared_arc(), worker),
            chain_end_bits: Bits::attach(self.chain_end_bits.shared_arc(), worker),
            taken_then_bits: Bits::attach(self.taken_then_bits.shared_arc(), worker),
            taken_else_bits: Bits::attach(self.taken_else_bits.shared_arc(), worker),
            widened_bits: Bits::attach(self.widened_bits.shared_arc(), worker),
            opaque_bits: Bits::attach(self.opaque_bits.shared_arc(), worker),
            by_name_arg_bits: Bits::attach(self.by_name_arg_bits.shared_arc(), worker),
            spread_bits: Bits::attach(self.spread_bits.shared_arc(), worker),
            ascribed_bits: Bits::attach(self.ascribed_bits.shared_arc(), worker),
            inlined_null_bits: Bits::attach(self.inlined_null_bits.shared_arc(), worker),
            cast_bits: Bits::attach(self.cast_bits.shared_arc(), worker),
            setter_assign_bits: Bits::attach(self.setter_assign_bits.shared_arc(), worker),
            removed_bits: Bits::attach(self.removed_bits.shared_arc(), worker),
            tuple_index_bits: Bits::attach(self.tuple_index_bits.shared_arc(), worker),
            kept_bits: Bits::attach(self.kept_bits.shared_arc(), worker),
            leaf_bits: Bits::attach(self.leaf_bits.shared_arc(), worker),
            leaf_tests: FxMap::default(),
            in_place_operands: FxMap::default(),
            stored_tests: FxMap::default(),
            stored_pats: FxMap::default(),
        }
    }
}

pub use crate::types::NO_TYPE;
pub const NO_SPAN: (FileId, Span) = (FileId(u32::MAX), Span { start: 0, end: 0 });

impl Program {
    /// Whether `getClass` is used anywhere: every class then carries its qualified name.
    pub fn uses_get_class(&self) -> bool {
        !self.get_class_units.is_empty()
    }
}

/// The head of a chain of `+` (`Program::chain_head`).
#[derive(Clone, Copy, Debug)]
pub enum ChainHead {
    /// An operand like the others: evaluated, and rendered with them.
    Operand,
    /// The chain goes on in these operands, the first of them its head in turn.
    Chain(ListRef),
    /// The statements run where the head stands, and the chain goes on in the result.
    Block(ListRef, TExprId),
}

impl TExpr {
    /// The node's kind and what it names where it names a member, a class or a default (`AccessorNote::shape`): the
    /// same for a copy of the node, whatever its operands are.
    pub fn reference_shape(self) -> (std::mem::Discriminant<TExpr>, u32) {
        let named = match self {
            TExpr::CallMethod(_, s, _) | TExpr::Field(_, s) | TExpr::Static(s) | TExpr::CallStatic(s, _) => s.0,
            TExpr::New(c, _) => c.0,
            TExpr::Default(_, at, _) => at,
            _ => u32::MAX,
        };
        (std::mem::discriminant(&self), named)
    }

    /// The operand and the kind of the rendering, where the node is a concatenation's own
    /// rendering of an operand; `None` for what is evaluated as it stands, the program's
    /// `x.toString` included.
    #[inline]
    pub fn rendering(self) -> Option<(TExprId, StrKind)> {
        match self {
            TExpr::ToStr(inner, conv) if conv.is_rendering() => Some((inner, conv.kind())),
            _ => None,
        }
    }

    /// Whether the node is a literal.
    pub fn is_constant(self) -> bool {
        matches!(self, TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit)
    }
}

impl Default for Program {
    fn default() -> Program {
        Program {
            exprs: Arena::new(),
            pats: Arena::new(),
            strings: Arena::new(),
            expr_lists: Arena::new(),
            pat_lists: Arena::new(),
            sym_lists: Arena::new(),
            stmts: Arena::new(),
            cases: Arena::new(),
            tries: Arena::new(),
            tests: Arena::new(),
            deferred_tests: FxMap::default(),
            funs: Arena::new(),
            classes: Arena::new(),
            top_funs: Registers::new(),
            top_vals: Registers::new(),
            main: None,
            main_object: None,
            overrides: Default::default(),
            inherited_defaults: Default::default(),
            partial_function: None,
            class_tag_module: None,
            throwable: None,
            js_exception: None,
            js_imports: Vec::new(),
            js_exports: Vec::new(),
            record_types: false,
            types_only: false,
            capture: None,
            types_aside: 0,
            aside_types: FxMap::default(),
            template_calls: Vec::new(),
            get_class_units: Vec::new(),
            local_enum_names: Vec::new(),
            shifts: 0,
            build_start: 0,
            runs: 0,
            lifted_funs: Default::default(),
            block_class_traits: Default::default(),
            template_syms: Default::default(),
            class_bodies: Default::default(),
            expr_types: Parallel::new(NO_TYPE),
            expr_spans: Parallel::new(NO_SPAN),
            file_tags: crate::arena::FileVec::from_vec(Vec::new()),
            quotes: Arena::new(),
            quote_pats: Arena::new(),
            expansions: FxMap::default(),
            block_classes: FxMap::default(),
            forward: ForwardNotes::default(),
            expansion_bits: Bits::default(),
            chain_end_bits: Bits::default(),
            taken_then_bits: Bits::default(),
            taken_else_bits: Bits::default(),
            widened_bits: Bits::default(),
            opaque_bits: Bits::default(),
            by_name_arg_bits: Bits::default(),
            spread_bits: Bits::default(),
            ascribed_bits: Bits::default(),
            inlined_null_bits: Bits::default(),
            cast_bits: Bits::default(),
            setter_assign_bits: Bits::default(),
            removed_bits: Bits::lazy(),
            tuple_index_bits: Bits::default(),
            kept_bits: Bits::default(),
            leaf_bits: Bits::default(),
            leaf_tests: FxMap::default(),
            in_place_operands: FxMap::default(),
            stored_tests: FxMap::default(),
            stored_pats: FxMap::default(),
        }
    }
}

impl Program {
    /// The tag of a file appended after the program's files (a library body's pseudo file):
    /// its key's `body_tag`, 64 bits wide so that no two bodies of a build meet in one. Should
    /// two meet after all, the second to arrive moves past the tags in use, by the order of
    /// their conversions: the last resort, which a fresh build and a session that retains a
    /// body could resolve differently.
    pub fn push_file_tag(&mut self, key: &str) {
        let mut tag = crate::source::body_tag(key);
        while (0..self.file_tags.len()).any(|i| self.file_tags.get(i) == Some(&tag)) {
            tag = tag.wrapping_add(1);
        }
        self.file_tags.push(tag);
    }

    /// The tag of a product's pseudo file of top-level definitions: the token its producer
    /// recorded, as it is; a name it gives that another definition of the
    /// build also has is the emitter's collision to report, never displaced.
    pub fn push_recorded_tag(&mut self, token: u64) {
        self.file_tags.push(token);
    }

    pub fn file_init_name(&self, file: FileId) -> String {
        format!("$file{}", crate::source::tag_text(self.file_tags[file.0 as usize]))
    }

    /// `<file>_<offset>`, the position a fresh name takes, `<file>` the file's tag in base 36.
    pub fn position(&self, file: FileId, start: u32) -> String {
        format!("{}_{}", crate::source::tag_text(self.file_tags[file.0 as usize]), start)
    }

    /// Imports and exports only exist in ES modules; everything else stays a plain script.
    pub fn is_module(&self) -> bool {
        !self.js_imports.is_empty() || !self.js_exports.is_empty()
    }

    #[inline]
    pub fn add(&mut self, e: TExpr) -> TExprId {
        TExprId(self.exprs.push(e))
    }
    #[inline]
    pub fn set_type(&mut self, e: TExprId, ty: TypeId) {
        if self.record_types {
            if self.types_aside > 0 {
                self.aside_types.insert(e, ty);
                return;
            }
            if self.exprs.is_shared() {
                self.assert_writable(e, "type");
            }
            self.expr_types.set(e.0, ty);
        }
    }

    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn assert_writable(&self, e: TExprId, what: &str) {
        assert!(self.exprs.writable(e.0), "the {} of expression {} set by a worker that did not make it", what, e.0);
        #[cfg(debug_assertions)]
        self.exprs.check_writable_own(e.0);
    }

    /// Every record this worker made so far has escaped it (`Arena::escape_own`): the program's
    /// arenas and, with the expressions', the tables parallel to them.
    #[cfg(debug_assertions)]
    pub fn escape_own(&self) {
        self.exprs.escape_own();
        self.pats.escape_own();
        self.strings.escape_own();
        self.expr_lists.escape_own();
        self.pat_lists.escape_own();
        self.sym_lists.escape_own();
        self.stmts.escape_own();
        self.cases.escape_own();
        self.tries.escape_own();
        self.tests.escape_own();
        self.funs.escape_own();
        self.classes.escape_own();
        self.quotes.escape_own();
        self.quote_pats.escape_own();
    }

    /// Whether this worker made the node `e` and may record on it.
    #[inline]
    pub fn owns_expr(&self, e: TExprId) -> bool {
        !self.exprs.is_shared() || self.exprs.writable(e.0)
    }

    /// `e` with the type `ty` recorded: `e` itself where this worker may record it, else a copy
    /// of its node, which another worker reads as made (its maker's, or published).
    #[inline(always)]
    pub fn typed(&mut self, e: TExprId, ty: TypeId) -> TExprId {
        if self.record_types {
            if self.types_aside > 0 {
                self.aside_types.insert(e, ty);
                return e;
            }
            if self.exprs.is_shared() {
                return self.typed_forked(e, ty);
            }
            self.expr_types.set(e.0, ty);
        }
        e
    }

    #[inline(never)]
    fn typed_forked(&mut self, e: TExprId, ty: TypeId) -> TExprId {
        if self.exprs.writable(e.0) {
            self.set_type(e, ty);
            return e;
        }
        let copy = self.add(self.expr(e));
        self.copy_span(e, copy);
        self.copy_chain_marks(e, copy);
        self.set_type(copy, ty);
        copy
    }
    #[inline]
    pub fn expr(&self, id: TExprId) -> TExpr {
        *self.exprs.get(id.0)
    }
    /// Whether the typer records the source of each expression, with its type: not while it
    /// keeps the types of an inline body aside in a build that keeps none (`keep_types_aside`).
    #[inline]
    pub fn records_spans(&self) -> bool {
        self.record_types && self.types_aside == 0 && !self.types_only
    }

    /// Starts keeping the types the typer gives expressions aside in `aside_types`, in a build
    /// that keeps none; `true` when it did, for `end_types_aside`. Nested ones share the table.
    pub fn keep_types_aside(&mut self) -> bool {
        if self.record_types && self.types_aside == 0 {
            return false;
        }
        self.record_types = true;
        self.types_aside += 1;
        true
    }

    /// Ends what `keep_types_aside` started; the outermost drops the table.
    pub fn end_types_aside(&mut self) {
        self.types_aside -= 1;
        if self.types_aside == 0 {
            self.record_types = false;
            self.aside_types = FxMap::default();
        }
    }

    /// Puts back the type `type_or_aside` read of `e` before a trial retyped it in place (a
    /// number widened where no operation converts it), which a rollback does not undo.
    pub fn restore_type(&mut self, e: TExprId, ty: Option<TypeId>) {
        if !self.record_types || self.type_or_aside(e) == ty {
            return;
        }
        if self.types_aside > 0 {
            match ty {
                Some(t) => self.aside_types.insert(e, t),
                None => self.aside_types.remove(&e),
            };
            return;
        }
        if self.exprs.is_shared() {
            self.assert_writable(e, "type");
        }
        self.expr_types.set(e.0, ty.unwrap_or(NO_TYPE));
    }

    /// The type an expression was given, in the program's table or in the one kept aside.
    pub fn type_or_aside(&self, e: TExprId) -> Option<TypeId> {
        match self.types_aside {
            0 => self.type_of(e),
            _ => self.aside_types.get(&e).copied(),
        }
    }

    /// The type the typer recorded for an expression, when it did.
    #[inline]
    pub fn type_of(&self, e: TExprId) -> Option<TypeId> {
        self.expr_types.get(e.0).filter(|&t| t != NO_TYPE)
    }
    #[inline]
    pub fn set_span(&mut self, e: TExprId, file: FileId, span: Span) {
        if self.exprs.is_shared() {
            self.assert_writable(e, "source");
        }
        self.expr_spans.set(e.0, (file, span));
    }
    /// The test JavaScript makes for `t`: a class test its operand's type proves is the test that
    /// it is not `null`, or none (`TypeTest::Proven`).
    #[inline]
    pub fn js_test(&self, t: TestId) -> TypeTest {
        match self.tests[t.idx()] {
            TypeTest::Proven(_, true) => TypeTest::AnyVal,
            TypeTest::Proven(_, false) => TypeTest::Always,
            // Both sides proven (`Box[Int] & Mark`): one null test, or none.
            TypeTest::And(a, b) => match (self.js_test(a), self.js_test(b)) {
                (TypeTest::Always, TypeTest::Always) => TypeTest::Always,
                (TypeTest::Always | TypeTest::AnyVal, TypeTest::Always | TypeTest::AnyVal) => TypeTest::AnyVal,
                _ => TypeTest::And(a, b),
            },
            // A union's sides, each proven: one null test, or none.
            TypeTest::Or(a, b) => match (self.js_test(a), self.js_test(b)) {
                (TypeTest::Always, _) | (_, TypeTest::Always) => TypeTest::Always,
                (TypeTest::AnyVal, TypeTest::AnyVal) => TypeTest::AnyVal,
                _ => TypeTest::Or(a, b),
            },
            other => other,
        }
    }
    /// The source an expression was typed from, when it was recorded.
    pub fn span_of(&self, e: TExprId) -> Option<(FileId, Span)> {
        self.expr_spans.get(e.0).filter(|s| s.0 != NO_SPAN.0)
    }
    /// Gives `to`, a node made in place of `from`, the source of `from`.
    pub fn copy_span(&mut self, from: TExprId, to: TExprId) {
        if let Some((f, s)) = self.span_of(from) {
            self.set_span(to, f, s);
        }
    }
    pub fn note_expansion(&mut self, e: TExprId, x: Expansion) {
        self.expansions.insert(e, x);
        self.mark_expansion(e);
    }

    pub fn mark_expansion(&mut self, e: TExprId) {
        self.expansion_bits.set(e.0);
    }

    #[inline]
    pub fn is_expansion(&self, e: TExprId) -> bool {
        self.expansion_bits.has(e.0)
    }

    pub fn mark_chain_end(&mut self, e: TExprId) {
        self.chain_end_bits.set(e.0);
    }

    /// The condition of the `if` `e` is folded away and leaves the first branch, or the second.
    pub fn mark_taken(&mut self, e: TExprId, first: bool) {
        if first {
            self.taken_then_bits.set(e.0);
        } else {
            self.taken_else_bits.set(e.0);
        }
    }

    /// Gives `to`, a copy of `from`, what `from` says of the chain it heads.
    pub fn copy_chain_marks(&mut self, from: TExprId, to: TExprId) {
        if self.ends_chain(from) {
            self.mark_chain_end(to);
        }
        if let Some(first) = self.taken(from) {
            self.mark_taken(to, first);
        }
    }

    /// Marks `e` as `widened_bits` says.
    pub fn mark_widened(&mut self, e: TExprId) {
        self.widened_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_widened(&self, e: TExprId) -> bool {
        self.widened_bits.has(e.0)
    }

    /// Marks `e` as `by_name_arg_bits` says.
    pub fn mark_by_name_arg(&mut self, e: TExprId) {
        self.by_name_arg_bits.set(e.0);
    }

    pub fn is_by_name_arg(&self, e: TExprId) -> bool {
        self.by_name_arg_bits.has(e.0)
    }

    /// Marks `e` as `opaque_bits` says.
    pub fn mark_opaque(&mut self, e: TExprId) {
        self.opaque_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_opaque(&self, e: TExprId) -> bool {
        self.opaque_bits.has(e.0)
    }

    /// Marks `e` as `spread_bits` says.
    pub fn mark_spread(&mut self, e: TExprId) {
        self.spread_bits.set(e.0);
    }

    /// Marks `e` as `ascribed_bits` says.
    pub fn mark_ascribed(&mut self, e: TExprId) {
        self.ascribed_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_ascribed(&self, e: TExprId) -> bool {
        self.ascribed_bits.has(e.0)
    }

    /// Marks `e` as `inlined_null_bits` says.
    pub fn mark_inlined_null(&mut self, e: TExprId) {
        self.inlined_null_bits.set(e.0);
    }

    #[inline]
    pub fn is_inlined_null(&self, e: TExprId) -> bool {
        self.inlined_null_bits.has(e.0)
    }

    /// Marks `e` as a cast's tree (`cast_bits`).
    pub fn mark_cast(&mut self, e: TExprId) {
        self.cast_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_cast(&self, e: TExprId) -> bool {
        self.cast_bits.has(e.0)
    }

    #[inline(always)]
    pub fn is_spread(&self, e: TExprId) -> bool {
        self.spread_bits.has(e.0)
    }

    /// Whether `e` is an operand `Program::in_place_operands` holds.
    pub fn operand_in_place(&self, e: TExprId) -> bool {
        self.in_place_operands.contains_key(&e)
    }

    /// Marks `e` as `setter_assign_bits` says.
    pub fn mark_setter_assign(&mut self, e: TExprId) {
        self.setter_assign_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_setter_assign(&self, e: TExprId) -> bool {
        self.setter_assign_bits.has(e.0)
    }

    /// Marks the block statement `e`, or the result `e` of an ascribed block that is a statement, as one
    /// the reference's `PureStats` removes (`removed_bits`).
    pub fn mark_removed(&mut self, e: TExprId) {
        self.removed_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_removed(&self, e: TExprId) -> bool {
        self.removed_bits.has(e.0)
    }

    /// Whether a block's statement is run: every one but an expression `PureStats` removes.
    #[inline(always)]
    pub fn runs(&self, s: &TStmt) -> bool {
        !matches!(*s, TStmt::Expr(x) if self.is_removed(x))
    }

    /// A block's statements as a back end runs them (`runs`).
    pub fn run_stmts(&self, stmts: ListRef) -> impl Iterator<Item = TStmt> + '_ {
        self.stmts[stmts.range()].iter().copied().filter(|s| self.runs(s))
    }

    /// Marks `e` as `tuple_index_bits` says.
    pub fn mark_tuple_index(&mut self, e: TExprId) {
        self.tuple_index_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_tuple_index(&self, e: TExprId) -> bool {
        self.tuple_index_bits.has(e.0)
    }

    /// Marks `e` as a plain inline call kept for the inlining phase (`kept_bits`).
    pub fn mark_kept(&mut self, e: TExprId) {
        self.kept_bits.set(e.0);
    }

    #[inline(always)]
    pub fn is_kept(&self, e: TExprId) -> bool {
        self.kept_bits.has(e.0)
    }

    /// Whether `e` is an operand of its own where it heads a chain of `+`.
    #[inline]
    pub fn ends_chain(&self, e: TExprId) -> bool {
        self.chain_end_bits.has(e.0)
    }

    /// The branch that stands for the `if` `e`: the first (`true`) or the second.
    #[inline]
    pub fn taken(&self, e: TExprId) -> Option<bool> {
        if self.taken_then_bits.has(e.0) {
            Some(true)
        } else if self.taken_else_bits.has(e.0) {
            Some(false)
        } else {
            None
        }
    }

    /// What the head `e` of a chain of `+` is to the chain, as scalac's JVM backend finds its
    /// chains (`liftStringConcat`, after the phases that put a block's statements in front and
    /// a constant condition's branch in the place of its `if`): every backend that renders a
    /// chain after its operands asks this and nothing else.
    pub fn chain_head(&self, mut e: TExprId) -> ChainHead {
        loop {
            if self.ends_chain(e) {
                return ChainHead::Operand;
            }
            match self.expr(e) {
                TExpr::StrConcat(items) => return ChainHead::Chain(items),
                TExpr::Block(stmts, res) if self.continues_chain(res) => return ChainHead::Block(stmts, res),
                TExpr::If(_, first, second) => match (self.taken(e), second) {
                    (Some(true), _) => e = first,
                    (Some(false), Some(second)) => e = second,
                    _ => return ChainHead::Operand,
                },
                _ => return ChainHead::Operand,
            }
        }
    }

    #[inline]
    pub fn continues_chain(&self, e: TExprId) -> bool {
        match self.expr(e) {
            TExpr::StrConcat(_) | TExpr::Block(..) | TExpr::If(..) => !matches!(self.chain_head(e), ChainHead::Operand),
            _ => false,
        }
    }

    pub fn note_leaf(&mut self, e: TExprId) {
        self.leaf_bits.set(e.0);
    }

    #[inline]
    pub fn is_leaf(&self, e: TExprId) -> bool {
        self.leaf_bits.has(e.0)
    }

    /// What a concatenation or an interpolation makes of its operand `inner`, which is no string.
    pub fn rendering(&mut self, inner: TExprId, kind: StrKind) -> TExprId {
        self.add(TExpr::ToStr(inner, StrConv::rendering(kind)))
    }

    /// `inner.toString` as the program wrote it.
    pub fn to_string_call(&mut self, inner: TExprId, kind: StrKind) -> TExprId {
        self.add(TExpr::ToStr(inner, StrConv::call(kind)))
    }

    /// Whether the node is a string by construction, never `null`, on which the program's
    /// `toString` is the string itself: a literal, a concatenation, a rendering, or a `toString`
    /// call of a value of no user's class (a number's, a String's, which returns itself). A
    /// call of a reference's `toString` returns what its method returns, `null` included.
    pub fn is_non_null_string(&self, e: TExprId) -> bool {
        match self.expr(e) {
            TExpr::Str(_) | TExpr::StrConcat(_) => true,
            TExpr::ToStr(_, conv) => conv.is_rendering() || conv.kind() != StrKind::Generic,
            _ => false,
        }
    }

    #[inline]
    pub fn add_pat(&mut self, p: TPat) -> TPatId {
        TPatId(self.pats.push(p))
    }
    #[inline]
    pub fn add_str(&mut self, s: &str) -> StrRef {
        StrRef(self.strings.push(s.to_string()))
    }
    #[inline]
    pub fn add_test(&mut self, t: TypeTest) -> TestId {
        TestId(self.tests.push(t))
    }
    #[inline]
    pub fn add_fun(&mut self, f: TFun) -> FunId {
        FunId(self.funs.push(f))
    }
    pub fn list(&mut self, items: &[TExprId]) -> ListRef {
        self.expr_lists.push_slice(items)
    }
    pub fn syms(&mut self, items: &[SymId]) -> ListRef {
        self.sym_lists.push_slice(items)
    }
    #[inline]
    pub fn expr_list(&self, l: ListRef) -> &[TExprId] {
        self.expr_lists.slice(l.start, l.len)
    }
    #[inline]
    pub fn sym_list(&self, l: ListRef) -> &[SymId] {
        self.sym_lists.slice(l.start, l.len)
    }
    #[inline]
    pub fn stmt_list(&self, l: ListRef) -> &[TStmt] {
        self.stmts.slice(l.start, l.len)
    }
    #[inline]
    pub fn case_list(&self, l: ListRef) -> &[TCase] {
        self.cases.slice(l.start, l.len)
    }
    #[inline]
    pub fn pat_list(&self, l: ListRef) -> &[TPatId] {
        self.pat_lists.slice(l.start, l.len)
    }

    /// The def that the lambda `(a, b) => f(a, b)` passes its parameters to, in order, which the
    /// JavaScript output writes as `f` itself.
    pub fn forwarded_target(&self, params: ListRef, body: TExprId) -> Option<SymId> {
        let TExpr::CallStatic(target, args) = self.expr(body) else { return None };
        let params = self.sym_list(params);
        let args = self.expr_list(args);
        let forwards = !params.is_empty()
            && params.len() == args.len()
            && params.iter().zip(args).all(|(&p, &a)| matches!(self.expr(a), TExpr::Local(s) if s == p));
        forwards.then_some(target)
    }

    /// The receiver (when it is not `this`) and the arguments of `e` when it calls the def `sym`
    /// with all of its `arity` parameters: what a tail call needs to become a jump.
    pub fn self_call(&self, sym: SymId, arity: usize, e: TExprId) -> Option<(Option<TExprId>, ListRef)> {
        let (recv, args) = match self.expr(e) {
            TExpr::CallStatic(s, args) if s == sym => (None, args),
            TExpr::CallMethod(r, s, args) if s == sym => {
                (if matches!(self.expr(r), TExpr::This) { None } else { Some(r) }, args)
            }
            _ => return None,
        };
        let items = self.expr_list(args);
        let complete = items.len() == arity && !items.iter().any(|&a| matches!(self.expr(a), TExpr::Spread(_)));
        complete.then_some((recv, args))
    }

    /// The `return`s of the def `sym`'s body `body` whose value holds a call of the def by itself
    /// with all its `arity` parameters in a tail position (`tail_self_calls`'s positions), and those
    /// calls: dotty's TailRec takes a return from the method as a tail position wherever it stands
    /// (TailRec.scala 474-478), so each call is the def's loop going round again. Not in a closure's
    /// or a local def's code, which is another method, and not where a `try`'s body, a case of a
    /// `try` with a finalizer or a `synchronized` body guards the `return`: a jump there skips the
    /// finalizer, an enclosing handler or the monitor's exit, where scalac's does
    /// (docs/DIFFERENCES.md).
    pub fn tail_jumps(&self, syms: &Symbols, sym: SymId, arity: usize, body: TExprId) -> TailJumps {
        let mut out = TailJumps::default();
        // Each expression with whether nothing guards it.
        let mut stack: Vec<(TExprId, bool)> = vec![(body, true)];
        while let Some((e, free)) = stack.pop() {
            // An inline method's by-name argument is a proxy def of dotty's expansion, whose code is
            // none of the method's own (TailRec.scala 450-461 enters no nested DefDef).
            if self.is_by_name_arg(e) {
                continue;
            }
            match self.expr(e) {
                TExpr::Return(v) => {
                    if free {
                        let before = out.calls.len();
                        self.tail_calls(sym, arity, v, &mut out.calls);
                        if out.calls.len() > before {
                            out.returns.push(e);
                        }
                    }
                    stack.push((v, free));
                }
                TExpr::Lambda(..) => {}
                TExpr::Js(template, args) if self.strings[template.idx()] == "$synchronized($0, $1)" => {
                    let &[lock, inner] = self.expr_list(args) else { continue };
                    stack.push((lock, free));
                    stack.push((inner, false));
                }
                TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::TypeTest(r, _, _) | TExpr::Cast(r, ..) | TExpr::Index(r, _)
                | TExpr::Spread(r) | TExpr::JsSelect(r, _) | TExpr::Throw(r, _) => stack.push((r, free)),
                TExpr::CallStatic(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args)
                | TExpr::ArrayLit(args) | TExpr::ObjLit(args) | TExpr::Default(_, _, args) | TExpr::New(_, args) => {
                    stack.extend(self.expr_list(args).iter().map(|&a| (a, free)));
                }
                TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                    stack.push((r, free));
                    stack.extend(self.expr_list(args).iter().map(|&a| (a, free)));
                }
                TExpr::If(c, t, els) => {
                    stack.push((c, free));
                    stack.push((t, free));
                    stack.extend(els.map(|x| (x, free)));
                }
                TExpr::While(a, b) | TExpr::Assign(a, b) | TExpr::Prim(_, a, b) => {
                    stack.push((a, free));
                    stack.push((b, free));
                }
                TExpr::Block(stmts, res) => {
                    for st in &self.stmts[stmts.range()] {
                        match *st {
                            TStmt::Expr(x) | TStmt::Pat(_, x, _) => stack.push((x, free)),
                            // A lazy val's initialiser runs in a method of its own (LazyVals, after
                            // TailRec, which takes its right-hand side as no tail position).
                            TStmt::Val(s, _) if syms.sym(s).mods & mods::LAZY != 0 => {}
                            TStmt::Val(_, init) => stack.push((init, free)),
                            TStmt::Fun(_) => {}
                        }
                    }
                    stack.push((res, free));
                }
                TExpr::Match(scrut, cases) => {
                    stack.push((scrut, free));
                    for c in self.case_list(cases) {
                        stack.extend(c.guard.map(|g| (g, free)));
                        stack.push((c.body, free));
                    }
                }
                TExpr::Try(i) => {
                    let t = &self.tries[i as usize];
                    let cases_free = free && t.finalizer.is_none();
                    stack.push((t.body, false));
                    for c in self.case_list(t.cases) {
                        stack.extend(c.guard.map(|g| (g, cases_free)));
                        stack.push((c.body, cases_free));
                    }
                    stack.extend(t.finalizer.map(|f| (f, free)));
                }
                _ => {}
            }
        }
        out
    }

    /// The calls of `sym` by itself in the tail positions of `e`, as `tail_self_calls` counts them.
    pub fn tail_calls(&self, sym: SymId, arity: usize, e: TExprId, out: &mut Vec<TExprId>) {
        match self.expr(e) {
            TExpr::Block(_, res) | TExpr::Prim(PrimOp::BoolAnd | PrimOp::BoolOr, _, res) | TExpr::Return(res) => self.tail_calls(sym, arity, res, out),
            TExpr::If(_, t, els) => {
                self.tail_calls(sym, arity, t, out);
                if let Some(x) = els {
                    self.tail_calls(sym, arity, x, out);
                }
            }
            TExpr::Match(_, cases) => {
                for c in self.case_list(cases) {
                    self.tail_calls(sym, arity, c.body, out);
                }
            }
            TExpr::Try(i) if self.tries[i as usize].finalizer.is_none() => {
                for c in self.case_list(self.tries[i as usize].cases) {
                    self.tail_calls(sym, arity, c.body, out);
                }
            }
            _ => {
                if self.self_call(sym, arity, e).is_some() && !out.contains(&e) {
                    out.push(e);
                }
            }
        }
    }

    /// The tail positions of `e` (the value of a block, the branches of an `if`, the bodies of
    /// the cases of a `match` or of a `catch` without `finally`, the right operand of `&&` and
    /// `||`) that hold such a call, and whether one of them has a receiver other than `this`.
    pub fn tail_self_calls(&self, sym: SymId, arity: usize, e: TExprId) -> (usize, bool) {
        let both = |a: (usize, bool), b: (usize, bool)| (a.0 + b.0, a.1 || b.1);
        match self.expr(e) {
            TExpr::Block(_, res) | TExpr::Prim(PrimOp::BoolAnd | PrimOp::BoolOr, _, res) | TExpr::Return(res) => {
                self.tail_self_calls(sym, arity, res)
            }
            TExpr::If(_, t, els) => {
                let then = self.tail_self_calls(sym, arity, t);
                els.map_or(then, |x| both(then, self.tail_self_calls(sym, arity, x)))
            }
            TExpr::Match(_, cases) => self.case_list(cases)
                .iter()
                .fold((0, false), |acc, c| both(acc, self.tail_self_calls(sym, arity, c.body))),
            // The handler of a `try` runs once the body is over, so its cases are tail positions
            // unless a finalizer follows them, as scalac has it.
            TExpr::Try(i) if self.tries[i as usize].finalizer.is_none() => self.case_list(self.tries[i as usize].cases)
                .iter()
                .fold((0, false), |acc, c| both(acc, self.tail_self_calls(sym, arity, c.body))),
            _ => match self.self_call(sym, arity, e) {
                Some((recv, _)) => (1, recv.is_some()),
                None => (0, false),
            },
        }
    }
}

/// The `return`s of a def's body that go round the def's loop, and their calls of the def
/// (`Program::tail_jumps`).
#[derive(Default)]
pub struct TailJumps {
    pub returns: Vec<TExprId>,
    pub calls: Vec<TExprId>,
}

/// The expressions under a root, the root included, in no particular order: the bodies of the
/// lambdas and local defs inside it, the guards and bodies of its cases, the expressions of its
/// patterns and type tests.
pub struct Descendants<'a> {
    prog: &'a Program,
    exprs: Vec<TExprId>,
    pats: Vec<TPatId>,
    tests: Vec<TestId>,
    local_defs: bool,
    patterns: bool,
}

impl Program {
    pub fn descendants(&self, root: TExprId) -> Descendants<'_> {
        Descendants { prog: self, exprs: vec![root], pats: Vec::new(), tests: Vec::new(), local_defs: true, patterns: true }
    }
}

impl<'a> Descendants<'a> {
    /// The walk without the bodies of the local defs, which run apart from their block.
    pub fn without_local_defs(mut self) -> Self {
        self.local_defs = false;
        self
    }

    /// The walk over the nodes a copy of a tree makes anew: without the bodies of the local
    /// defs and the expressions of the patterns and type tests, which the copy has in common
    /// with the tree it copies (`Worker::copy_expr`).
    pub fn of_a_copy(mut self) -> Self {
        self.local_defs = false;
        self.patterns = false;
        self
    }

    fn push_cases(&mut self, l: ListRef) {
        for case in &self.prog.cases[l.range()] {
            self.pats.push(case.pat);
            self.exprs.extend(case.guard);
            self.exprs.push(case.body);
        }
    }

    fn drain_pats_and_tests(&mut self) -> bool {
        let prog = self.prog;
        while self.exprs.is_empty() {
            if let Some(p) = self.pats.pop() {
                match prog.pats[p.idx()] {
                    TPat::Wildcard => {}
                    TPat::Bind(_, inner) => self.pats.extend(inner),
                    TPat::Test(test, _, inner) => {
                        self.tests.push(test);
                        self.pats.push(inner);
                    }
                    TPat::Equals(e, _) => self.exprs.push(e),
                    TPat::Class(_, _, _, subs) | TPat::Alt(subs) => self.pats.extend_from_slice(&prog.pat_lists[subs.range()]),
                    TPat::Seq(items, rest) => {
                        self.pats.extend_from_slice(&prog.pat_lists[items.range()]);
                        self.pats.extend(rest);
                    }
                    TPat::Unapply(_, call, inner) => {
                        self.exprs.push(call);
                        self.pats.push(inner);
                    }
                }
            } else if let Some(t) = self.tests.pop() {
                match prog.tests[t.idx()] {
                    TypeTest::Value(e) => self.exprs.push(e),
                    TypeTest::Or(a, b) | TypeTest::And(a, b) => self.tests.extend([a, b]),
                    TypeTest::Outer(_, t) | TypeTest::Proven(t, _) => self.tests.push(t),
                    _ => {}
                }
            } else {
                return false;
            }
        }
        true
    }
}

impl<'a> Iterator for Descendants<'a> {
    type Item = TExprId;

    fn next(&mut self) -> Option<TExprId> {
        if !self.patterns {
            self.pats.clear();
            self.tests.clear();
        }
        if !self.drain_pats_and_tests() {
            return None;
        }
        let prog = self.prog;
        let e = self.exprs.pop()?;
        match prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_)
            | TExpr::Unit | TExpr::Local(_) | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_)
            | TExpr::ClassOf(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::Null => {}
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::Index(r, _) | TExpr::Spread(r)
            | TExpr::JsSelect(r, _) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::Lambda(_, r) | TExpr::Splice(r) => {
                self.exprs.push(r)
            }
            TExpr::TypeTest(r, test, _) | TExpr::Cast(r, CastOp::Check(test, _) | CastOp::Unbox(test, _) | CastOp::Adapt(test, _), _) => {
                self.exprs.push(r);
                self.tests.push(test);
            }
            TExpr::Cast(r, CastOp::Written | CastOp::Nothing | CastOp::Erased, _) => self.exprs.push(r),
            TExpr::CallStatic(_, args) | TExpr::New(_, args) | TExpr::NewVia(_, args) | TExpr::StrConcat(args)
            | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) | TExpr::Default(_, _, args) => {
                self.exprs.extend_from_slice(prog.expr_list(args));
            }
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.exprs.push(r);
                self.exprs.extend_from_slice(prog.expr_list(args));
            }
            TExpr::If(c, t, els) => {
                self.exprs.extend([c, t]);
                self.exprs.extend(els);
            }
            TExpr::While(a, b) | TExpr::Prim(_, a, b) | TExpr::Assign(a, b) => self.exprs.extend([a, b]),
            TExpr::Block(stmts, res) => {
                self.exprs.push(res);
                for s in &prog.stmts[stmts.range()] {
                    match *s {
                        TStmt::Expr(e) | TStmt::Val(_, e) => self.exprs.push(e),
                        TStmt::Pat(p, e, _) => {
                            self.pats.push(p);
                            self.exprs.push(e);
                        }
                        TStmt::Fun(f) if self.local_defs => self.exprs.extend(prog.funs[f.idx()].body),
                        TStmt::Fun(_) => {}
                    }
                }
            }
            TExpr::Match(scrut, l) => {
                self.exprs.push(scrut);
                self.push_cases(l);
            }
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                self.exprs.push(t.body);
                self.exprs.extend(t.finalizer);
                self.push_cases(t.cases);
            }
        }
        Some(e)
    }
}

macro_rules! plain_records {
    ($($t:ty),*) => { $(impl crate::arena::Record for $t {})* };
}
plain_records!(TExpr, TPat, TExprId, TPatId, TStmt, TCase, TTry, TypeTest, TFun, TClass, TQuote, TQuotePat);

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::*;

    /// Two workers over one fork, worker 1 on a thread it leaves before worker 0 reads: worker 1
    /// makes an expression and records its type, and publishes it when `escape`, its records
    /// escaping and the expression sealed (what a publication of it does).
    fn forked(escape: bool) -> (Program, Program, TExprId) {
        let mut main = Program { record_types: true, ..Program::default() };
        main.fork();
        let mut peer = main.attach(1);
        let e = std::thread::scope(|s| {
            s.spawn(|| {
                let e = peer.add(TExpr::Unit);
                peer.set_type(e, TypeId(7));
                if escape {
                    peer.escape_own();
                    peer.exprs.seal(e.0);
                }
                e
            })
            .join()
            .unwrap()
        });
        (main, peer, e)
    }

    fn refusal(f: impl FnOnce()) -> String {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => String::new(),
            Err(e) => e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(),
        }
    }

    /// A peer's read of an expression's type alone takes part in the read protocol: it marks
    /// the expression read, so that its owner's later `set_type` is refused, as a read of the
    /// expression itself would.
    #[test]
    fn an_owners_type_write_after_a_peers_type_read_is_refused() {
        let (main, mut peer, e) = forked(true);
        assert_eq!(main.type_of(e), Some(TypeId(7)));
        let refused = refusal(|| peer.set_type(e, TypeId(8)));
        assert!(refused.contains("after a peer read it"), "the owner's second type was not refused: {:?}", refused);
    }

    /// A peer reads an expression's type only once the expression escaped its owner.
    #[test]
    fn a_peers_type_read_before_the_escape_is_refused() {
        let (main, _peer, e) = forked(false);
        let refused = refusal(|| {
            main.type_of(e);
        });
        assert!(refused.contains("before it escaped"), "the type read before the escape was not refused: {:?}", refused);
    }
}

/// Whether `s` is the lazy val of a companion the typer made for a case class nested in a class
/// (`Namer`'s, `Desugar.classDef`'s `companionDefs`), which has no body to run.
pub fn is_made_companion_val(syms: &crate::symbols::Symbols, s: SymId) -> bool {
    companion_val_of_case(syms, s).is_some_and(|m| syms.class(m).made_companion)
}

/// Whether `s` is the lazy val of a companion the typer or the parser made for a case class nested
/// in a class (`Desugar.classDef`'s `companionDefs`), which has no body to run: on JavaScript it is
/// written, and the companion reached, only where something reads it.
pub fn is_desugared_companion_val(syms: &crate::symbols::Symbols, s: SymId) -> bool {
    companion_val_of_case(syms, s).is_some_and(|m| syms.class(m).made_companion || syms.class(m).synthetic_companion)
}

/// Whether `s` is the val, lazy or eager, of the companion the namer made for a case class or an
/// enum's class case nested in its owner (`ClassInfo::made_companion`), which the rules of the
/// backends and the writer lay out with its class rather than as a member.
pub fn is_made_case_companion_val(syms: &crate::symbols::Symbols, s: SymId) -> bool {
    use crate::symbols::{Owner, SymKind};
    let info = syms.sym(s);
    if info.kind != SymKind::Val {
        return false;
    }
    let Owner::Class(owner) = info.owner else { return false };
    let Some(&k) = syms.class(owner).nested.get(&info.name) else { return false };
    if syms.class(k).mods & crate::ast::mods::CASE == 0 {
        return false;
    }
    syms.class(k).companion.is_some_and(|m| syms.class(m).inner_object == Some(s) && syms.class(m).made_companion && syms.class(m).def.is_none())
}

/// The companion whose lazy val `s` is, of a case class nested in a class.
fn companion_val_of_case(syms: &crate::symbols::Symbols, s: SymId) -> Option<crate::types::ClassId> {
    use crate::symbols::{Owner, SymKind};
    let info = syms.sym(s);
    if info.kind != SymKind::Val || info.mods & crate::ast::mods::LAZY == 0 {
        return None;
    }
    let Owner::Class(owner) = info.owner else { return None };
    let &k = syms.class(owner).nested.get(&info.name)?;
    if syms.class(k).mods & crate::ast::mods::CASE == 0 {
        return None;
    }
    syms.class(k).companion.filter(|&m| syms.class(m).inner_object == Some(s))
}
