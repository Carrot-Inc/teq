//! The lowered IR of the class-based targets: a method body after the platform lowering
//! (erasure, boxing, the choice of each member's reference), as a tree of nodes that leave
//! values of erased types, and the erased names it speaks in: types, and the references to
//! classes and members that stand where an erased symbol table would. The JVM back end writes
//! bytecode from it; a node the lowering does not make yet is `Legacy`, which the JVM back
//! end generates from the typed IR as it did before (docs/LIR.md).

use crate::source::FileId;
use crate::tir::{StrRef, TExprId, TestId};
use crate::types::{ClassId, SymId, TypeId};
use std::rc::Rc;

/// An erased type: a primitive, or a class or interface by its binary name with `/` between
/// the packages (an array is its element's descriptor behind `[`). `V` is no value: the result
/// of a method that returns `Unit`. The variants are spelled as the class file's descriptors
/// spell them (docs/LIR.md, "The vocabulary").
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ErasedType {
    V,
    Z,
    B,
    S,
    C,
    I,
    J,
    F,
    D,
    L(Rc<str>),
}

/// How a method is reached: through the receiver's class, through an interface, with no
/// receiver, or the named class's own (a constructor, a private method, a `super` call).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Invoke {
    Virtual,
    Interface,
    Static,
    Special,
}

/// An erased method: the class that declares it, its encoded name and its erased signature.
#[derive(Clone)]
pub struct MethodRef {
    pub owner: Rc<str>,
    pub owner_is_interface: bool,
    pub name: String,
    pub params: Vec<ErasedType>,
    pub ret: ErasedType,
    /// The signature as a class file spells it, made once with the reference.
    pub desc: String,
    pub kind: Invoke,
}

/// An operation on two numbers of one primitive type, or on two booleans (`And`, `Or`, `Xor`).
/// A shift's distance is an int.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Ushr,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Relation {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

/// How the two operands of a comparison are compared.
#[derive(Clone)]
pub enum Comparing {
    /// As references: the same object or not; `eq` and `ne`, and `==` of a class, where a `null`
    /// side is tested alone (scalac's `genEqEqPrimitive` and the reference ops of
    /// `genComparisonOp`, `BCodeBodyBuilder.genCond` 1578-1650 at 3.8.4).
    Identity,
    /// As references, the same object or not, where a `null` side is loaded as the other is: `==`
    /// and `!=` of an operand that is no class (an array, whose `equals` is identity), which scalac
    /// compares as `genComparisonOp` does a universal equality (the same lines).
    ArrayEquality,
    /// As numbers of the operands' type, or as booleans for equality. A NaN is in no order
    /// with anything and equal to nothing.
    Numbers,
    /// An `Int` by its sign, the right operand `Int(0)` and not written: the left alone and one
    /// `ifXX`. teq's own tests of what its helpers give (`seqPattern`'s parts, empty for no
    /// match), which no comparison of a program lowers to: a program's `x == 0` is `Numbers`
    /// (`iconst_0; if_icmpXX`, scalac's `genCJUMP`; `BCodeBodyBuilder.genComparisonOp` at 3.8.4
    /// folds no comparison with zero).
    Sign,
    /// By the sign of what this method of the left operand gives for the right one
    /// (`Comparable.compareTo`).
    Ordering(Rc<MethodRef>),
    /// Equal where this static method says so of the two.
    Equality(Rc<MethodRef>),
    /// Two unit values, which are equal.
    Units,
}

/// What a cast does to its operand (`Kind::Cast`), as erasure leaves it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cast {
    /// Nothing: erasure dropped the cast, the operand's erased type conforming to the
    /// target's. The node's type is the operand's.
    Erased,
    /// Nothing, of a reference whose target no class tests: `Object`, a function of more
    /// parameters than `Function22` has, and a value class, whose box is tested where the value
    /// is adapted. The node's type is `Object`.
    Unchecked,
    /// The test that a reference is null or an instance of the node's type, a class, an
    /// interface or an array; any other reference throws `ClassCastException`.
    Checked,
    /// The primitive of the node's type out of a reference: zero for null, the content of that
    /// primitive's box, and `ClassCastException` for any other reference.
    Unbox,
    /// A cast to `Nothing`: the operand is evaluated and dropped, then a `ClassCastException`
    /// with the message `Cannot cast to scala.Nothing` is thrown. What follows the node is not
    /// reached; its type is the one that code is written against.
    Nothing,
}

/// What an adaptation does to a value (`Kind::Adapt`), by the type it has and the type wanted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Adaptation {
    /// The value is dropped.
    Drop,
    /// There was no value: the unit value stands for it.
    Unit,
    /// There was no value: the zero of the primitive wanted stands for it.
    Zero,
    /// A reference as a class it is an instance of by its own class: nothing happens.
    Widen,
    /// A reference as a class it is tested to be null or an instance of; any other reference
    /// throws `ClassCastException`.
    Narrow,
    /// The first character of a string.
    CharOfString,
    /// The string of one character.
    StringOfChar,
    /// The primitive out of a reference: zero for null, the content of that primitive's box, and
    /// `ClassCastException` for any other reference.
    Unbox,
    /// The primitive in its box.
    Box,
    /// A number as another primitive type, by Java's conversions between them.
    Convert,
}

/// A method the lowering makes of a closure's body or of a local definition (lambda lifting): a
/// method of the class that takes no receiver and, in this order, the instance its code runs
/// on, the key of the non-local returns of the def around it, the captured values, then its own
/// parameters. It has no name here: a writer names it.
pub struct LiftedShape {
    /// The type the instance is taken at, where the code names it.
    pub instance: Option<ErasedType>,
    /// Whether the key is taken, a reference.
    pub key: bool,
    /// The types the captured values are taken at.
    pub captured: Vec<ErasedType>,
    pub params: Vec<ErasedType>,
    pub ret: ErasedType,
}

impl LiftedShape {
    /// Everything the method takes, in its order.
    pub fn takes(&self) -> Vec<ErasedType> {
        let mut all: Vec<ErasedType> = self.instance.iter().cloned().collect();
        if self.key {
            all.push(ErasedType::object());
        }
        all.extend(self.captured.iter().cloned());
        all.extend(self.params.iter().cloned());
        all
    }
}

/// What a lifted method a `LiftedCall` calls was lifted from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LiftedFrom {
    /// A local definition: a method of the class being written.
    Def,
    /// A local lazy val, its accessor: a method of the class whose code declares the val, taking
    /// the val's holder last.
    LazyAccessor,
    /// A local lazy val, its initialiser, as the accessor.
    LazyInitialiser,
}

/// A captured value as the code of a lifted method names it: the local `sym` of the declared
/// type `ty`, held in its reference cell where `cell`; where `key`, not the local but the key of
/// the non-local returns that travels with the local lazy val `sym`. Transitional with
/// `LiftedBody`.
#[derive(Clone)]
pub struct Capture {
    pub sym: SymId,
    pub cell: bool,
    pub ty: ErasedType,
    pub key: bool,
}

/// What the generator of the typed IR writes a lifted method's body from: a closure's
/// parameters and body (`code`; a local definition's are its symbol's), the locals the captured
/// values are in that code, the class whose instance the code runs on where that is not the
/// class being written (`home`), and the object that stands for that instance where it is read
/// with no receiver (`module`). Transitional: it ends when lifted bodies are methods of the
/// class-level plan, their parameters carrying these roles.
pub struct LiftedBody {
    pub code: Option<(Vec<SymId>, TExprId)>,
    pub captures: Vec<Capture>,
    pub home: Option<ClassId>,
    pub module: Option<Rc<str>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LirId(pub u32);

/// A temporary of the lowering (`Kind::Let`, `Kind::Temp`), numbered per method by `Body::temp`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TempId(pub u32);

/// A labelled block of the lowering (`Kind::Labeled`, `Kind::Break`), numbered per method by
/// `Body::label`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LabelId(pub u32);

/// Nodes in order, a slice of `Body::lists`.
#[derive(Clone, Copy, Debug)]
pub struct List {
    pub start: u32,
    pub len: u32,
}

pub struct Node {
    pub kind: Kind,
    /// The erased type of the value the node leaves, `ErasedType::V` for none.
    pub ty: ErasedType,
    /// The expression of the typed IR the node is the whole of, whose position its code takes.
    pub of: Option<TExprId>,
}

/// Why a form of the typed IR is not lowered yet: what an escape node (`Kind::Legacy`,
/// `Kind::LegacyTail`) carries, so that the forms the old generator still writes
/// are this list (docs/LIR.md, "How the extraction goes").
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Unlowered {
    /// `val (a, b) = p`, a definition by a pattern, among a block's statements.
    PatternDefinition,
    /// `return x` where it is not the def's result: a statement, an operand, inside a `try`.
    ReturnInValuePosition,
    /// `return x` inside a lambda, which leaves the def around it by a throw.
    NonLocalReturn,
    /// A lambda, a local def's call, a local lazy val's read, `new` or a default of a class that
    /// captures, or a local trait's member call, after an earlier part of the method was left to
    /// the old generator: what it captures is asked in the order the code is written.
    AfterEscape,
    /// A right-associative member of a local trait that takes what the trait captures.
    LocalTraitMemberRightAssociative,
    /// A member of a local trait that takes what the trait captures, called on `super`, or with
    /// a value for a capture that is no read of a local.
    LocalTraitMemberUnplain,
    /// A local def's default argument.
    DefaultOfLocalDef,
    /// A local `def`'s declaration, where its lifted method is named and queued (`Kind::LocalFun`).
    LocalDefDeclaration,
    /// `class B extends A` where every parameter of `A` has a default and none is written.
    CtorSuperDefaults,
    /// A class body whose local trait's initialiser takes values of the enclosing block.
    CtorTraitProxies,
    /// An enum companion's `fromOrdinal` over its stored values.
    EnumCompanionMembers,
    /// A value class's method forwarding to its `m$extension`, and the copies of extension methods.
    ValueClassForwarder,
    /// The function a by-name default of a jar's method is passed as.
    JarGetterThunk,
    /// The super accessor of a local trait's member that takes what the trait captures.
    ProxiedSuperAccessor,
    /// An object's `<clinit>` own code: the handles of its lazy members, its instance made and
    /// stored in `MODULE$`.
    ModuleClinit,
    /// A lambda's or local def's parameter that its lifted method takes as an `Object`, opened
    /// into a local of its own type before the body.
    LiftedParameterPrologue,
    /// A local `def` without a body (reported).
    LocalDefWithoutBody,
}

#[derive(Clone)]
pub enum Kind {
    /// An expression of the typed IR brought to the node's type, not lowered yet, and why.
    Legacy(TExprId, Unlowered),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    Char(u16),
    Str(StrRef),
    /// A string constant by its text, where the lowering made the text (the constants a
    /// concatenation starts with, which the typer folds).
    Text(Rc<str>),
    Unit,
    /// The null reference: the null literal, or a null the lowering makes.
    Null,
    /// A local or a parameter held in the method's frame, read at the node's type: its value,
    /// or the reference cell that holds it.
    Local(SymId),
    /// A parameter of the method by its position among the declared ones (the receiver is none
    /// of them): how a body the back end composes names what it was given, since its
    /// parameters have no symbol.
    Param(u32),
    /// Statements in order, then the result, whose value is the block's. The locals declared in
    /// it end with it.
    Block { stmts: List, result: LirId },
    /// A local declared with its first value. `cell`, for a local that a closure captures and
    /// that is assigned, is the type declared for it: the value held is then its reference cell.
    Val { sym: SymId, init: LirId, cell: Option<ErasedType> },
    /// A local held as its value assigned.
    Assign { sym: SymId, value: LirId },
    /// A local definition is declared here: nothing is evaluated. A writer that names lifted
    /// methods in the order it meets them names this one here, where no call came first.
    /// Transitional until a local definition is a method of the class-level plan.
    LocalFun(SymId),
    /// The instance of an object, by its class's name, at the node's type: the class's own, or
    /// a type the class implements (a given object's declared type), taken with no check.
    Module(Rc<str>),
    /// The class value of the erased type: a class's or an interface's, an array's (its
    /// descriptor holds the dimensions), a primitive's, and `Unit`'s for `V`. The class-file
    /// writer reads a primitive's from its box's class.
    ClassOf(ErasedType),
    /// The instance the method runs on, at the node's type.
    This,
    /// A field of an instance read, named on the class `owner`.
    Field { receiver: LirId, owner: Rc<str>, name: Rc<str> },
    /// An operation on two values of the node's type.
    Binary { op: Arith, left: LirId, right: LirId },
    /// A number negated.
    Neg(LirId),
    /// A boolean negated.
    Not(LirId),
    /// Both hold; the second is evaluated only where the first holds.
    And(LirId, LirId),
    /// One holds; the second is evaluated only where the first does not.
    Or(LirId, LirId),
    /// A boolean: the relation between two operands.
    Compare { relation: Relation, how: Comparing, left: LirId, right: LirId },
    /// A boolean: a type test of the program that erasure makes constant (`x.isInstanceOf[Matchable]`,
    /// a class no program of the build makes), on a reference, written by `Gen::test`, which folds
    /// it. Transitional: a block in a condition whose result is a constant is written as the
    /// constant tested (`Gen::write_cond`), which this test's temporary in a block would be; it
    /// ends when the two take one form.
    Test { value: LirId, test: TestId, tested: TypeId },
    /// The runtime's helper applied: the arguments are evaluated in order, then the helper's
    /// object, which is not an operand.
    Helper { method: Rc<MethodRef>, args: List },
    /// `then` where the condition holds, `otherwise` where it does not; with no `otherwise`
    /// nothing is left.
    If { cond: LirId, then: LirId, otherwise: Option<LirId> },
    /// The body, for as long as the condition holds before it; with no condition, for ever (dotty's
    /// `WhileDo(EmptyTree, body)`, which only the compiler makes: nothing follows it).
    While { cond: Option<LirId>, body: LirId },
    /// The exception thrown. Nothing follows it: the node's type is the one its place wants.
    Throw(LirId),
    /// The def's result: the value is returned.
    Return(LirId),
    /// The def called by itself where it loops, in return position or in a tail position of a
    /// `return`'s value: the receiver where another instance is called, then the arguments, in
    /// order; they become `this` and the parameters, and the body starts again. Nothing follows
    /// it: the node's type is the one its place wants, the operands under it dropped; as a
    /// condition (`&&`'s or `||`'s right operand) nothing tests it.
    SelfTailCall { receiver: Option<LirId>, args: List },
    /// A block not lowered, in return position, and why: the generator of the typed IR writes it,
    /// each of its results returned. Transitional.
    LegacyTail(TExprId, Unlowered),
    /// A field of an instance assigned.
    FieldStore { receiver: LirId, owner: Rc<str>, name: Rc<str>, value: LirId },
    /// A class's static field read.
    StaticField { owner: Rc<str>, name: Rc<str> },
    /// A class's static field assigned, the value at the field's type. No value.
    StaticFieldStore { owner: Rc<str>, name: Rc<str>, value: LirId },
    /// A call of a method on a receiver: the receiver, then the arguments as they are written,
    /// each at the type its parameter takes. `at` is the class the reference names where that
    /// is the receiver's and not the declaring one's (dotty's `receiverClass`), with whether it
    /// is an interface. `order`, where the method takes its parameters in another order than
    /// the arguments are written and evaluated in (a right-associative extension), is the
    /// argument each parameter takes.
    Call { method: Rc<MethodRef>, at: Option<(Rc<str>, bool)>, receiver: LirId, args: List, order: Option<Rc<[usize]>> },
    /// `super.m(..)`: the method `method` names, the named class's own, called on the receiver
    /// with no dispatch on its class; receiver and arguments as `Call` has them. `place` is the
    /// end of the call in its source file, which orders the interfaces a class file lists for
    /// the super calls of its code (`Gen::super_call_place`); a NIR writer reads none.
    SuperCall { method: Rc<MethodRef>, receiver: LirId, args: List, order: Option<Rc<[usize]>>, place: Option<(FileId, u32)> },
    /// A value class's underlying value selected on a value of the class: `value`, which the
    /// lowering brought to the underlying type. `operand` is the expression of the typed IR
    /// selected on: while it may be `Legacy`, its generator is told through it that the box is
    /// opened with no null test (`Gen::fit`). Transitional until no such operand is `Legacy`.
    Underlying { value: LirId, operand: TExprId },
    /// A new instance of the class `ctor` is the constructor of, the constructor run on it over
    /// the arguments, each a value its parameter takes: at the parameter's type, or as the scope
    /// holds it (what a lifted class captures, an inner class's enclosing instance), at a type
    /// the parameter's takes. `touch` is the companion object that is initialised before the
    /// instance is made.
    New { touch: Option<Rc<str>>, ctor: Rc<MethodRef>, args: List },
    /// The constructor `ctor` run on the instance being made, over the arguments at its
    /// parameters' types: a superclass's, or another of the class's own, in a constructor.
    InitCall { ctor: Rc<MethodRef>, args: List },
    /// The operand's value as the node's, with no operation: an expression that is its operand
    /// seen at the node's type, which the operand's type represents as the node's does (a value
    /// class's instance made of its constructor's argument, held as that underlying value; a
    /// default argument that is its expression where the class has no getter; a null that an
    /// ascription holds, dotty's `Typed` over the literal).
    Typed(LirId),
    /// A call of a method that takes no receiver: the arguments as `Call` has them.
    StaticCall { method: Rc<MethodRef>, args: List, order: Option<Rc<[usize]>> },
    /// A new array, of the node's type, that holds the items in order, each evaluated at the
    /// type `elem` of its elements. The class-file writer makes the array before it evaluates
    /// the items, which nothing observes.
    ArrayOf { elem: ErasedType, items: List },
    /// An element of an array read: the array, then the index, an `Int`. The node's type is the
    /// type of the array's elements.
    ArrayLoad { array: LirId, index: LirId },
    /// An element of an array assigned: the array, then the index, an `Int`, then the value, at
    /// the type of the array's elements. No value.
    ArrayStore { array: LirId, index: LirId, value: LirId },
    /// The number of elements of an array, an `Int`.
    ArrayLength(LirId),
    /// A new array of the array's type holding its elements.
    ArrayClone(LirId),
    /// A new array of the node's type with `length` elements, an `Int`, each the zero of the
    /// type of its elements (null for a reference).
    NewArray { length: LirId },
    /// `value.asInstanceOf[T]`, by what the cast does (`Cast`).
    Cast { value: LirId, op: Cast },
    /// `value` as a string, as `java.lang.String.valueOf` makes one of a value of its type: a
    /// primitive's text (a `Byte`'s and a `Short`'s as an `Int`'s), `()` for no value, a
    /// `String` as it is (a null one stays null), another reference's `toString`, and `null`
    /// for a null one.
    StringOf { value: LirId },
    /// The concatenation of the parts, in order. Every part is evaluated, left to right, before
    /// the first is made a string; each is then the string Java's string conversion gives a
    /// value of its type (a primitive's text, `null` for a null reference, a `String` or else
    /// its `toString`). A part that is a `Str` or a `Text` is a constant of the result. How
    /// many calls make the string, and what is text of their recipes, is the class-file
    /// writer's.
    Concat { parts: List },
    /// `try`: the body; where it throws, the cases (`Catch`); the finalizer, which runs on every
    /// way out. `returns`: whether a `return` of the method leaves through the finalizer
    /// (`Body::returns_through`), which the JVM writer must know before the `try` to make the
    /// method's early-return slot (dotty's `shouldEmitCleanup`). Recorded by the lowering, which
    /// sees into the typed IR of the escape nodes inside.
    Try { body: LirId, catch: Option<Catch>, finalizer: Option<LirId>, returns: bool },
    /// The body's value, evaluated holding the monitor of the object `lock`, which is evaluated
    /// first; the monitor is released on every way out of the body, a throw and a `return`
    /// included. `returns`: whether a `return` of the method leaves through the monitor exit,
    /// as `Try`'s.
    Synchronized { lock: LirId, body: LirId, returns: bool },
    /// A value of the type `from` as one of the node's type: dropped, the unit value or a zero
    /// for none, a cast, a box or its content, a primitive conversion (`Gen::adapt`).
    Adapt { value: LirId, from: ErasedType, how: Adaptation },
    /// A function object: an instance of the interface of `method` whose `method` calls the
    /// lifted method `lifted` with the captured values, then its own arguments, and gives its
    /// result. `captures` are those values as `lifted` takes them (the instance, the key, the
    /// captured ones), evaluated here in that order. The node's type is the interface. A writer
    /// makes the lifted method and names it (a class file's `$anonfun$N` is numbered as the
    /// closures are written, a node written twice making two). `body` is what the method is
    /// made of, transitional (`LiftedBody`).
    Closure { captures: List, method: Rc<MethodRef>, lifted: Rc<LiftedShape>, body: Rc<LiftedBody> },
    /// A call of a lifted method (`from`): the local definition `def`'s, or the accessor or the
    /// initialiser of the local lazy val `def`: the captured values as `Closure` has them, then the
    /// arguments as `Call` has them. `def` is transitional: the writer names the method from it,
    /// and finds the class of a lazy val's, until lifted bodies are methods of the class-level plan.
    LiftedCall { def: SymId, from: LiftedFrom, lifted: Rc<LiftedShape>, captures: List, args: List, order: Option<Rc<[usize]>> },
    /// The key of the non-local returns of the def the code stands in, a reference: the one the
    /// def made, or the one the code was given where it is a closure's.
    ReturnKey,
    /// The key of the non-local returns that travels with the holder of a local lazy val, where
    /// the code holds it: given to the method, held by the instance, or the def's own
    /// (`ReturnKey`). Transitional: the symbol ends when a method's hidden parameters are
    /// parameters of the class-level plan's method.
    LazyKey(SymId),
    /// The instance of a class that encloses the code, which the method was given beside its
    /// parameters. Transitional as `LazyKey` is.
    GivenInstance(ClassId),
    /// The temporary declared with the value of `init`; it lives to the end of the block that
    /// holds this node. No value.
    Let { temp: TempId, init: LirId },
    /// The slots of the temporary taken here, before the `Let` that gives it its value, of the type `ty` (two for a
    /// long or a double): a definition whose own slot comes before its right side's (dotty's `val $1` of a pattern
    /// definition, `Gen::lower_tuple_definition`; the one variable of its general match, `Kind::LocalAt`), as dotty
    /// makes a val's local before it loads the right side (`getOrMakeLocal`, BCodeBodyBuilder.scala 331-339 at
    /// 3.8.4). No value.
    Reserve { temp: TempId, ty: ErasedType },
    /// The local `sym`, of the type `ty`, held from here in the slot of the temporary: the one variable of a
    /// pattern definition's general match, `val x = rhs match { case pat => x }`, whose slot a `Reserve` took
    /// before its right side and a `Let` filled (`Gen::lower_variables_definition`); the pattern's own binder of
    /// the same symbol, inside the match, holds another. No value, no instruction.
    LocalAt { sym: SymId, temp: TempId, ty: ErasedType },
    /// A new value of the temporary a `Let` made. No value.
    SetTemp { temp: TempId, value: LirId },
    /// A boolean: a lazy member's container, the field `field` of the class `owner` on `object`, set to `new` where
    /// it holds `expected`, at once (dotty's `casFlag` over the field, LazyVals.scala 365-369). The field is an
    /// `Object`; how the operation is spelled is the writer's (the JVM's: the field's `VarHandle`, which the class's
    /// lazy member holds).
    LazyCas { object: LirId, owner: Rc<str>, field: Rc<str>, expected: LirId, new: LirId },
    /// The value of the temporary.
    Temp(TempId),
    /// A boolean: whether the reference is an instance of the class, interface or array class
    /// `class`; false for null.
    Is { value: LirId, class: Rc<str> },
    /// The body's value, or the value a `Break` to `label` inside it leaves the block with.
    Labeled { label: LabelId, body: LirId },
    /// The labelled block `label` left with `value`, none where the block leaves nothing.
    /// Nothing follows it: the node's type is the one its place wants. A conditional with no
    /// `otherwise` whose `then` is a `Break` without a value is a jump on the condition.
    Break { label: LabelId, value: Option<LirId> },
    /// A form the target has no translation of: the program is refused here, with the
    /// refusal's message, and nothing is evaluated. No value.
    Refused(Refusal),
}

/// Why a form is refused (`Kind::Refused`): one variant a form no back end of the class-based
/// targets translates, carrying what its message names.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// JavaScript interop: `js.Dynamic`, a JavaScript import or global, an object literal.
    JavaScriptInterop,
    /// A member of a JavaScript type.
    JavaScriptMember,
    /// An instance of a JavaScript class made.
    JavaScriptClass,
    /// A member with no `@jvm` template or body, by its name and the class that declares it
    /// where the build knows it.
    NoJvmBody { name: Rc<str>, owner: Option<Rc<str>> },
    /// A template text of the typer's that is JavaScript's alone.
    JavaScriptTemplate(Rc<str>),
    /// An enum's lookup (`values`, `valueOf`, `fromOrdinal`) on a value whose companion is not known.
    EnumLookupWithoutCompanion,
    /// A partial function literal in a build whose runtime has no `PartialFunctionImpl`.
    PartialFunctionWithoutRuntimeClass,
    /// A `@jvm` template's stack code no tree is read from, or one whose instructions do not take the
    /// values the use gives them: what is named (`instruction`, `placeholder`, `method reference`,
    /// `field reference`, `runtime call`), the item and the text.
    Template { what: &'static str, item: Rc<str>, text: Rc<str> },
    /// A template's new array of the use's type, which is no array's.
    NewArrayOfUnknownElement,
    /// `super.m(..)` of a program's member that carries its own `@jvm` template, which stands for every call of
    /// it: no class file method of it is the super call's.
    SuperCallOfOwnTemplate { member: Rc<str> },
    /// An enum case's call of its enum's constructor that passes none of the enum's arguments.
    EnumCaseWithoutArguments,
    /// A class's arguments to a trait that do not match the trait's parameters.
    TraitArgumentsMismatch,
    /// A by-name parameter's default whose getter returns the value while the parameter is read as a
    /// function: a class file with no Scala signature, where scalac sees no default at all.
    ByNameDefaultWithoutSignature,
    /// A default of a constructor whose owner is no class.
    DefaultOfConstructorOfNoClass,
    /// A new instance through a secondary constructor whose owner is no class.
    SecondaryConstructorOfNoClass,
    /// A default of a method of a library's value class whose companion's class file has no
    /// getter for it (`m$default$N$extension`), which scalac writes for every one.
    ValueClassDefaultWithoutGetter,
    /// A constructor's default whose getter takes a local the method being written does not hold:
    /// a local class's local companion, a capture of the class (a secondary constructor's default
    /// created away from the defining body; scalac passes the companion's holder along, which
    /// the typed tree does not name yet).
    LocalNotInScope(Rc<str>),
    /// A constructor's default of a local class that captures a value no local holds.
    DefaultCaptureOfThisKind,
    /// A constructor's default of an inner class whose enclosing instance the method being
    /// written does not hold.
    InstanceNotInScope(Rc<str>),
    /// Varargs in a build whose library has no `ArraySeq` teq reads (a Scala 2 scala-library alone
    /// on the class path).
    VarargsWithoutArraySeq,
    /// An assignment to a target that is no local, field or member.
    AssignmentTarget,
    /// A Java instance's field assigned with no receiver, which the typer does not make: an
    /// internal error.
    JavaFieldWithoutReceiver(Rc<str>),
    /// A read of an `inline val` with no constant type, which `InlineVals` refuses first.
    InlineValWithoutConstant(Rc<str>),
}

impl Refusal {
    /// The refusal as a back end reports it.
    pub fn message(&self) -> String {
        match self {
            Refusal::JavaScriptInterop => "JavaScript interop".to_string(),
            Refusal::JavaScriptMember => "a member of a JavaScript type".to_string(),
            Refusal::JavaScriptClass => "a JavaScript class".to_string(),
            Refusal::NoJvmBody { name, owner: Some(owner) } => format!("no @jvm intrinsic or body for {} of {}", name, owner),
            Refusal::NoJvmBody { name, owner: None } => format!("no @jvm intrinsic or body for {}", name),
            Refusal::JavaScriptTemplate(text) => format!("the JavaScript template {}", text),
            Refusal::EnumLookupWithoutCompanion => "an enum lookup whose companion is not known".to_string(),
            Refusal::PartialFunctionWithoutRuntimeClass => "the runtime class PartialFunctionImpl is missing".to_string(),
            Refusal::Template { what, item, text } => format!("the {} {} of {}", what, item, text),
            Refusal::NewArrayOfUnknownElement => "a new array of an element type only a ClassTag knows".to_string(),
            Refusal::SuperCallOfOwnTemplate { member } => format!("not supported on the JVM yet: a super call of {}, which carries its own `@jvm` template", member),
            Refusal::EnumCaseWithoutArguments => "an enum case without the arguments of its enum".to_string(),
            Refusal::TraitArgumentsMismatch => "the arguments of a trait that its parameters do not match".to_string(),
            Refusal::ByNameDefaultWithoutSignature => "the default of a by-name parameter of a class file without its Scala signature".to_string(),
            Refusal::DefaultOfConstructorOfNoClass => "the default of a constructor that is not in a class".to_string(),
            Refusal::SecondaryConstructorOfNoClass => "a secondary constructor that is not in a class".to_string(),
            Refusal::ValueClassDefaultWithoutGetter => "a default argument of a value class's extension method without its getter".to_string(),
            Refusal::LocalNotInScope(name) => format!("the local {} is not in scope of the generated method", name),
            Refusal::DefaultCaptureOfThisKind => "a default argument of a local class that captures this kind of value".to_string(),
            Refusal::InstanceNotInScope(name) => format!("the instance of {} where the generated method has none", name),
            Refusal::VarargsWithoutArraySeq => "varargs without ArraySeq".to_string(),
            Refusal::AssignmentTarget => "assignment to this kind of target".to_string(),
            Refusal::JavaFieldWithoutReceiver(name) => format!("internal error: the Java field {} assigned with no receiver", name),
            Refusal::InlineValWithoutConstant(name) => format!("internal error: the inline val {} has no constant type (InlineVals refuses it)", name),
        }
    }
}

impl Kind {
    /// Whether the node leaves typed IR to the generator that writes it where the node stands
    /// (`Body::typed_ir_left`). Every kind is listed, so that a new one is classified.
    fn leaves_typed_ir(&self) -> bool {
        match self {
            Kind::Legacy(..) | Kind::LegacyTail(..) | Kind::Test { .. } => true,
            Kind::Int(_) | Kind::Long(_) | Kind::Float(_) | Kind::Double(_) | Kind::Bool(_) | Kind::Char(_) | Kind::Str(_) | Kind::Text(_) | Kind::Unit | Kind::Null => false,
            Kind::Local(_) | Kind::Param(_) | Kind::Block { .. } | Kind::Val { .. } | Kind::Assign { .. } | Kind::LocalFun(_) | Kind::Module(_) | Kind::ClassOf(_) | Kind::This => false,
            Kind::Field { .. } | Kind::Binary { .. } | Kind::Neg(_) | Kind::Not(_) | Kind::And(..) | Kind::Or(..) | Kind::Compare { .. } => false,
            Kind::Helper { .. } | Kind::If { .. } | Kind::While { .. } | Kind::Throw(_) | Kind::Return(_) | Kind::SelfTailCall { .. } | Kind::Synchronized { .. } => false,
            Kind::Try { .. } => false,
            Kind::FieldStore { .. } | Kind::StaticField { .. } | Kind::Call { .. } | Kind::SuperCall { .. } | Kind::New { .. } | Kind::InitCall { .. } | Kind::StaticCall { .. } => false,
            Kind::StaticFieldStore { .. } => false,
            Kind::ArrayOf { .. } | Kind::ArrayLoad { .. } | Kind::Cast { .. } | Kind::StringOf { .. } | Kind::Concat { .. } => false,
            Kind::ArrayStore { .. } | Kind::ArrayLength(_) | Kind::ArrayClone(_) | Kind::NewArray { .. } => false,
            Kind::Adapt { .. } | Kind::Typed(_) => false,
            // Its operand is a node of its own, which says whether it leaves any.
            Kind::Underlying { .. } => false,
            // The body is queued, and the definition named: neither is written where the node stands.
            Kind::Closure { .. } | Kind::LiftedCall { .. } | Kind::ReturnKey | Kind::LazyKey(_) | Kind::GivenInstance(_) => false,
            Kind::Let { .. } | Kind::SetTemp { .. } | Kind::Reserve { .. } | Kind::LocalAt { .. } | Kind::Temp(_) | Kind::Is { .. } | Kind::Labeled { .. } | Kind::Break { .. } => false,
            Kind::Refused(_) | Kind::LazyCas { .. } => false,
        }
    }
}

/// A `try`'s cases as dotty's TryCatchPatterns leaves them (`Gen::lower_catch`): a handler for each
/// leading case a catch takes by the class of what it catches, then, where cases are left, one of
/// `java/lang/Throwable` whose body matches them and throws the exception again. A handler's body
/// leaves the `try` by a `Break` to `done` with the try's value (none for a `try` that leaves
/// nothing), or throws.
#[derive(Clone)]
pub struct Catch {
    pub done: LabelId,
    pub handlers: Rc<[ExceptionHandler]>,
}

/// A handler of a `try` (`Catch`): its body run for an exception of the class `class`, which it
/// holds as `caught` says.
#[derive(Clone)]
pub struct ExceptionHandler {
    pub class: Rc<str>,
    pub caught: Caught,
    pub body: LirId,
}

/// How a handler holds the exception it takes: dropped (dotty's `NamelessEH`), in the local of the
/// case's binder (`BoundEH`), or in a temporary, the binder TryCatchPatterns makes for the cases
/// matched in one handler.
#[derive(Clone, Copy)]
pub enum Caught {
    Dropped,
    Local(SymId),
    Temp(TempId),
}

/// The nodes of one method.
#[derive(Default)]
pub struct Body {
    pub nodes: Vec<Node>,
    pub lists: Vec<LirId>,
    /// Whether the lowering that runs has left typed IR to the generator that writes it
    /// (`Kind::leaves_typed_ir`): what that generator asks and names, it asks and names where
    /// the node is written, after everything lowered before it and before everything after.
    pub typed_ir_left: bool,
    temps: u32,
    labels: u32,
}

impl Body {
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.lists.clear();
        self.typed_ir_left = false;
        self.temps = 0;
        self.labels = 0;
    }

    pub fn temp(&mut self) -> TempId {
        self.temps += 1;
        TempId(self.temps - 1)
    }

    pub fn label(&mut self) -> LabelId {
        self.labels += 1;
        LabelId(self.labels - 1)
    }

    pub fn list(&mut self, items: &[LirId]) -> List {
        let start = self.lists.len() as u32;
        self.lists.extend_from_slice(items);
        List { start, len: items.len() as u32 }
    }

    pub fn push(&mut self, kind: Kind, ty: ErasedType) -> LirId {
        self.typed_ir_left |= kind.leaves_typed_ir();
        self.nodes.push(Node { kind, ty, of: None });
        LirId(self.nodes.len() as u32 - 1)
    }

    pub fn node(&self, id: LirId) -> &Node {
        &self.nodes[id.0 as usize]
    }

    pub fn ty(&self, id: LirId) -> &ErasedType {
        &self.nodes[id.0 as usize].ty
    }

    fn items(&self, list: List) -> impl Iterator<Item = LirId> + '_ {
        self.lists[list.start as usize..(list.start + list.len) as usize].iter().copied()
    }

    /// The nodes the node evaluates, in the order it lists them: a closure's captures (its body is
    /// another method's). Every kind is listed, so that
    /// a new one is classified.
    pub fn operands(&self, id: LirId) -> Vec<LirId> {
        let mut out = Vec::new();
        match &self.node(id).kind {
            Kind::Legacy(..) | Kind::LegacyTail(..) | Kind::LocalFun(_) => {}
            Kind::Int(_) | Kind::Long(_) | Kind::Float(_) | Kind::Double(_) | Kind::Bool(_) | Kind::Char(_) | Kind::Str(_) | Kind::Text(_) | Kind::Unit | Kind::Null => {}
            Kind::Local(_) | Kind::Param(_) | Kind::Module(_) | Kind::ClassOf(_) | Kind::This | Kind::StaticField { .. } => {}
            Kind::ReturnKey | Kind::LazyKey(_) | Kind::GivenInstance(_) | Kind::Temp(_) | Kind::Reserve { .. } | Kind::LocalAt { .. } | Kind::Refused(_) => {}
            Kind::Block { stmts, result } => {
                out.extend(self.items(*stmts));
                out.push(*result);
            }
            Kind::Val { init: value, .. } | Kind::Assign { value, .. } | Kind::StaticFieldStore { value, .. } | Kind::Let { init: value, .. } | Kind::SetTemp { value, .. } => out.push(*value),
            Kind::Field { receiver: value, .. } | Kind::Neg(value) | Kind::Not(value) | Kind::Test { value, .. } | Kind::Throw(value) | Kind::Return(value) => out.push(*value),
            Kind::Underlying { value, .. } | Kind::Typed(value) | Kind::ArrayLength(value) | Kind::ArrayClone(value) | Kind::NewArray { length: value } => out.push(*value),
            Kind::Cast { value, .. } | Kind::StringOf { value } | Kind::Adapt { value, .. } | Kind::Is { value, .. } => out.push(*value),
            Kind::LazyCas { object, expected, new, .. } => out.extend([*object, *expected, *new]),
            Kind::Labeled { body, .. } => out.push(*body),
            Kind::Binary { left, right, .. } | Kind::Compare { left, right, .. } | Kind::And(left, right) | Kind::Or(left, right) => out.extend([*left, *right]),
            Kind::ArrayLoad { array, index } => out.extend([*array, *index]),
            Kind::ArrayStore { array, index, value } => out.extend([*array, *index, *value]),
            Kind::FieldStore { receiver, value, .. } => out.extend([*receiver, *value]),
            Kind::Synchronized { lock, body, .. } => out.extend([*lock, *body]),
            Kind::While { cond, body } => {
                out.extend(*cond);
                out.push(*body);
            }
            Kind::If { cond, then, otherwise } => {
                out.extend([*cond, *then]);
                out.extend(*otherwise);
            }
            Kind::Helper { args, .. } | Kind::New { args, .. } | Kind::InitCall { args, .. } | Kind::StaticCall { args, .. } => out.extend(self.items(*args)),
            Kind::ArrayOf { items, .. } | Kind::Concat { parts: items } => out.extend(self.items(*items)),
            Kind::Call { receiver, args, .. } | Kind::SuperCall { receiver, args, .. } => {
                out.push(*receiver);
                out.extend(self.items(*args));
            }
            Kind::SelfTailCall { receiver, args } => {
                out.extend(*receiver);
                out.extend(self.items(*args));
            }
            Kind::Closure { captures, .. } => out.extend(self.items(*captures)),
            Kind::LiftedCall { captures, args, .. } => {
                out.extend(self.items(*captures));
                out.extend(self.items(*args));
            }
            Kind::Try { body, catch, finalizer, .. } => {
                out.push(*body);
                out.extend(catch.iter().flat_map(|c| c.handlers.iter().map(|h| h.body)));
                out.extend(*finalizer);
            }
            Kind::Break { value, .. } => out.extend(*value),
        }
        out
    }

    /// Whether a `return` of the method stands in the node's code, from where it leaves through
    /// every finalizer and monitor exit around it. An escape node's typed IR is not the tree's to
    /// see into: `escaped` answers for it.
    pub fn holds_return(&self, id: LirId, escaped: &dyn Fn(TExprId) -> bool) -> bool {
        match &self.node(id).kind {
            Kind::Return(_) => true,
            Kind::Legacy(e, _) | Kind::LegacyTail(e, _) => escaped(*e),
            _ => self.operands(id).into_iter().any(|o| self.holds_return(o, escaped)),
        }
    }

    /// Whether a `return` of the method leaves through the node: a `try` with a finalizer whose
    /// body or cases hold one, a `synchronized` block whose body does (`Body::holds_return`).
    /// Whether a `try` is the node or stands under it, as dotty's `mayCleanStack` finds one in a finalizer
    /// (BCodeSyncAndTry.scala 466-469): a closure's or a lifted def's body, a method of its own, is not under it.
    pub fn holds_try(&self, id: LirId) -> bool {
        matches!(self.node(id).kind, Kind::Try { .. }) || self.operands(id).into_iter().any(|o| self.holds_try(o))
    }

    pub fn returns_through(&self, id: LirId, escaped: &dyn Fn(TExprId) -> bool) -> bool {
        match &self.node(id).kind {
            Kind::Try { body, catch, finalizer: Some(_), .. } => {
                self.holds_return(*body, escaped) || catch.iter().flat_map(|c| c.handlers.iter()).any(|h| self.holds_return(h.body, escaped))
            }
            Kind::Synchronized { body, .. } => self.holds_return(*body, escaped),
            _ => false,
        }
    }

    /// Whether the node's code holds an escape node, whose typed IR the tree does not see into.
    pub fn holds_escape(&self, id: LirId) -> bool {
        matches!(self.node(id).kind, Kind::Legacy(..) | Kind::LegacyTail(..)) || self.operands(id).into_iter().any(|o| self.holds_escape(o))
    }
}

/// The nodes of a body a back end composes itself, where no typed IR stands behind the method
/// (a companion's `apply`, a bridge, a forwarder): each is made with the type it has.
impl Body {
    pub fn this(&mut self, ty: ErasedType) -> LirId {
        self.push(Kind::This, ty)
    }

    pub fn int(&mut self, v: i32) -> LirId {
        self.push(Kind::Int(v), ErasedType::I)
    }

    pub fn bool(&mut self, v: bool) -> LirId {
        self.push(Kind::Bool(v), ErasedType::Z)
    }

    pub fn unit(&mut self) -> LirId {
        self.push(Kind::Unit, ErasedType::V)
    }

    /// The string constant, at the string type `ty`.
    pub fn text(&mut self, text: &Rc<str>, ty: ErasedType) -> LirId {
        self.push(Kind::Text(text.clone()), ty)
    }

    pub fn field(&mut self, receiver: LirId, owner: &Rc<str>, name: &str, ty: ErasedType) -> LirId {
        self.push(Kind::Field { receiver, owner: owner.clone(), name: Rc::from(name) }, ty)
    }

    pub fn store(&mut self, receiver: LirId, owner: &Rc<str>, name: &str, value: LirId) -> LirId {
        self.push(Kind::FieldStore { receiver, owner: owner.clone(), name: Rc::from(name), value }, ErasedType::V)
    }

    /// A new instance through `ctor`, no companion initialised first.
    pub fn make(&mut self, ctor: &Rc<MethodRef>, args: &[LirId]) -> LirId {
        let args = self.list(args);
        self.push(Kind::New { touch: None, ctor: ctor.clone(), args }, ErasedType::L(ctor.owner.clone()))
    }

    pub fn init(&mut self, ctor: &Rc<MethodRef>, args: &[LirId]) -> LirId {
        let args = self.list(args);
        self.push(Kind::InitCall { ctor: ctor.clone(), args }, ErasedType::V)
    }

    /// A call on `receiver` of the method as the reference names it, the arguments in its order.
    pub fn call(&mut self, method: &Rc<MethodRef>, receiver: LirId, args: &[LirId]) -> LirId {
        let args = self.list(args);
        self.push(Kind::Call { method: method.clone(), at: None, receiver, args, order: None }, method.ret.clone())
    }

    /// The statements in order, then `result`.
    pub fn seq(&mut self, stmts: &[LirId], result: LirId) -> LirId {
        let stmts = self.list(stmts);
        let ty = self.ty(result).clone();
        self.push(Kind::Block { stmts, result }, ty)
    }

    pub fn ret(&mut self, value: LirId) -> LirId {
        self.push(Kind::Return(value), ErasedType::V)
    }

    /// The instance of the object whose class is `class`.
    pub fn module(&mut self, class: &Rc<str>) -> LirId {
        self.push(Kind::Module(class.clone()), ErasedType::L(class.clone()))
    }

    pub fn static_call(&mut self, method: &Rc<MethodRef>, args: &[LirId]) -> LirId {
        let args = self.list(args);
        self.push(Kind::StaticCall { method: method.clone(), args, order: None }, method.ret.clone())
    }

    pub fn compare(&mut self, relation: Relation, how: Comparing, left: LirId, right: LirId) -> LirId {
        self.push(Kind::Compare { relation, how, left, right }, ErasedType::Z)
    }

    /// `then` where `cond` holds; nothing is left.
    pub fn when(&mut self, cond: LirId, then: LirId) -> LirId {
        self.push(Kind::If { cond, then, otherwise: None }, ErasedType::V)
    }

    /// `value` thrown, in a place that wants `ty`.
    pub fn throw(&mut self, value: LirId, ty: ErasedType) -> LirId {
        self.push(Kind::Throw(value), ty)
    }

    /// The local `sym` declared with the value of `init`, held as that value.
    pub fn val(&mut self, sym: SymId, init: LirId) -> LirId {
        self.push(Kind::Val { sym, init, cell: None }, ErasedType::V)
    }
}

/// The nodes of the static initialisers of lazy members' handles and of an enum's values, of the
/// accessors of `super` calls, bridges and a value class's forwarders.
impl Body {

    /// The class value of `of`.
    pub fn class_of(&mut self, of: ErasedType) -> LirId {
        self.push(Kind::ClassOf(of), ErasedType::L(Rc::from("java/lang/Class")))
    }

    pub fn store_static(&mut self, owner: &Rc<str>, name: &str, value: LirId) -> LirId {
        self.push(Kind::StaticFieldStore { owner: owner.clone(), name: Rc::from(name), value }, ErasedType::V)
    }

    pub fn static_field(&mut self, owner: &Rc<str>, name: &str, ty: ErasedType) -> LirId {
        self.push(Kind::StaticField { owner: owner.clone(), name: Rc::from(name) }, ty)
    }
}

/// The nodes of the members a back end synthesizes for case classes and value classes:
/// `equals`, `hashCode`, `canEqual` and their tests.
impl Body {
    /// Whether `value` is an instance of `class`.
    pub fn is(&mut self, value: LirId, class: &Rc<str>) -> LirId {
        self.push(Kind::Is { value, class: class.clone() }, ErasedType::Z)
    }

    pub fn not(&mut self, value: LirId) -> LirId {
        self.push(Kind::Not(value), ErasedType::Z)
    }

    /// `value` cast to the class or interface `ty`, which it is tested to be null or an instance of.
    pub fn checked(&mut self, value: LirId, ty: ErasedType) -> LirId {
        self.push(Kind::Cast { value, op: Cast::Checked }, ty)
    }

    /// The temporary `temp` declared with the value of `init`.
    pub fn keep(&mut self, temp: TempId, init: LirId) -> LirId {
        self.push(Kind::Let { temp, init }, ErasedType::V)
    }

    /// The value of the temporary `temp`, of the type `ty` it was declared at.
    pub fn kept(&mut self, temp: TempId, ty: ErasedType) -> LirId {
        self.push(Kind::Temp(temp), ty)
    }

    /// The labelled block `label` around `body`, which leaves no value.
    pub fn labeled(&mut self, label: LabelId, body: LirId) -> LirId {
        self.push(Kind::Labeled { label, body }, ErasedType::V)
    }

    /// The labelled block `label` left with no value.
    pub fn leave(&mut self, label: LabelId) -> LirId {
        self.push(Kind::Break { label, value: None }, ErasedType::V)
    }
}

// ---- the class-level plan ----

/// Who may name a member: any class, the classes of its package, or its own class alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Visibility {
    Public,
    Package,
    Private,
}

/// What a method is beside its name and signature.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MethodFlags {
    pub visibility: Visibility,
    /// It takes no receiver.
    pub is_static: bool,
    pub is_final: bool,
    /// It has no body.
    pub is_abstract: bool,
    /// No definition of the source is this method.
    pub synthetic: bool,
    /// It answers a signature the class inherits by calling the class's method of another.
    pub bridge: bool,
}

impl MethodFlags {
    /// A public instance method with a body, as a definition of the source is one.
    pub const PUBLIC: MethodFlags = MethodFlags { visibility: Visibility::Public, is_static: false, is_final: false, is_abstract: false, synthetic: false, bridge: false };
    /// A public method a class declares and does not define.
    pub const ABSTRACT: MethodFlags = MethodFlags { is_abstract: true, ..MethodFlags::PUBLIC };
}

/// A method a class defines, where `MethodRef` is one that code names: its encoded name and
/// its erased signature, the types its callers pass and are answered with.
#[derive(Debug)]
pub struct MethodDef {
    pub name: Rc<str>,
    pub params: Vec<ErasedType>,
    pub ret: ErasedType,
    /// The signature as a class file spells it, kept with the definition as `MethodRef::desc`
    /// is with a reference: a class is asked by it whether it holds a method.
    pub desc: String,
    pub flags: MethodFlags,
}

/// What a field is beside its name and type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FieldFlags {
    pub visibility: Visibility,
    /// The class holds it, not each instance.
    pub is_static: bool,
    pub is_final: bool,
    /// A read sees the last write of any thread.
    pub volatile: bool,
    /// No definition of the source is this field.
    pub synthetic: bool,
    /// It holds a value of the enum its class is the companion of.
    pub enum_value: bool,
}

impl FieldFlags {
    /// A private field of an instance, as a val's or a var's is.
    pub const PRIVATE: FieldFlags = FieldFlags { visibility: Visibility::Private, is_static: false, is_final: false, volatile: false, synthetic: false, enum_value: false };
}

/// A field a class defines.
#[derive(Debug)]
pub struct FieldDef {
    pub name: Rc<str>,
    pub ty: ErasedType,
    pub flags: FieldFlags,
}

/// What kind of class a class is. Every class the back ends write is public.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ClassFlags {
    /// A trait: its methods are implemented by the classes that extend it.
    pub interface: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    /// An enum's class, or the class of a case of it that takes parameters.
    pub is_enum: bool,
}

impl ClassFlags {
    /// A class no other extends.
    pub const FINAL: ClassFlags = ClassFlags { interface: false, is_abstract: false, is_final: true, is_enum: false };
}

/// A class as a writer of a class-based target receives it, apart from its code.
#[derive(Default)]
pub struct ClassDef {
    /// The binary name, `/` between the packages.
    pub name: Rc<str>,
    pub flags: ClassFlags,
    pub superclass: Rc<str>,
    /// The interfaces its parents give it, in their order: the traits it extends that no other
    /// of them brings, then those its kind gives it (a case class's `Product`, a mirror's). A
    /// class file names more: a module class's `java/io/Serializable`, and each interface a
    /// `super` call of the class's code goes to (`Gen::super_targets`), which the class-file
    /// writer adds once the code is written.
    pub interfaces: Vec<Rc<str>>,
    /// The name of the source file it is written from, where it has one.
    pub source: Option<Rc<str>>,
    /// In the order they are declared, but a lazy member's handle before every field declared
    /// before it. The list is whole when the class's last method is: a handle is declared where
    /// its member's accessor is written, an enum's values where its companion's initialiser
    /// stores them.
    pub fields: Vec<FieldDef>,
    /// In the order they are written.
    pub methods: Vec<MethodDef>,
}

impl ClassDef {
    /// Whether the class holds a method of this name and signature (`MethodDef::desc`).
    pub fn has_method(&self, name: &str, desc: &str) -> bool {
        self.methods.iter().any(|m| &*m.name == name && m.desc == desc)
    }

    pub fn has_method_named(&self, name: &str) -> bool {
        self.methods.iter().any(|m| &*m.name == name)
    }
}
