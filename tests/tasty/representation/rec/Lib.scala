// The oracle's nonrecursive RECtype (README.md), its first stage: a macro making a RecursiveType through
// `RecursiveType.apply` (QuotesImpl.scala 2286-2290, dotty's `RecType.apply`), whose body names no self, the one
// RECtype without a RECthis a source can make scalac pickle.
package repr.rec

import scala.quoted.*

/** `Kinds.make`'s annotation, refining so that an inferred type keeps it. */
class kinds(val x: Any) extends scala.annotation.StaticAnnotation with scala.annotation.RefiningAnnotation

/** `Pickled.forms`'s annotation, on a declared type. */
class shape(val x: Any) extends scala.annotation.StaticAnnotation

/** An annotation on a definition inside an annotation's argument (`Pickled.localAnnot`). */
class tagged(val n: Int) extends scala.annotation.StaticAnnotation

/** A string annotation whose payloads the products' tags must keep apart (the review's probes on 8aa2ab15a). */
class text(val x: String, val y: String = "") extends scala.annotation.StaticAnnotation

object Flag:
  val flag: Boolean = false
  inline def inl(x: Int): Int = x + 1

/** The detection pair of an annotation tree's constant kinds (README.md, "The checks"): `Int @kinds(if Flag.flag then
 *  1 else 2)` with Byte constants (`bytes`) or Int ones, a tree no source writes with Byte leaves but no conversion
 *  (the review's probe, out/codex-probes/type-representation-push-510f15f2f/Make.scala). */
object Kinds:
  transparent inline def make(inline bytes: Boolean): Any = ${ makeImpl('bytes) }

  def makeImpl(bytes: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    def leaf(n: Int): Term = Literal(if bytes.valueOrAbort then ByteConstant(n.toByte) else IntConstant(n))
    val arg = If('{ Flag.flag }.asTerm, leaf(1), leaf(2))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of an annotation tree's binding graph: two nested locals both named `x` (7 and 8), the
   *  reference the outer one's (`outer`) or the inner one's (the review's probe,
   *  out/codex-probes/type-representation-push-5feaa3987/Make.scala). */
  transparent inline def locals(inline outer: Boolean): Any = ${ localsImpl('outer) }

  def localsImpl(outer: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val owner = Symbol.spliceOwner
    val a = Symbol.newVal(owner, "x", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val b = Symbol.newVal(owner, "x", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val arg = Block(List(ValDef(a, Some(Literal(IntConstant(7))))),
      Block(List(ValDef(b, Some(Literal(IntConstant(8))))), Ref(if outer.valueOrAbort then a else b)))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a lambda type's parameter names: the refinement's `def f(x: Int, y: String): Int` (`first`)
   *  or `def f(z: Int, y: String): Int`, whose names the records after each parameter type hold (the review's probe on
   *  8aa2ab15a, Param.scala). */
  transparent inline def paramNames(inline first: Boolean): Any = ${ paramNamesImpl('first) }

  def paramNamesImpl(first: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val mt = MethodType(List(if first.valueOrAbort then "x" else "z", "y"))(
      _ => List(TypeRepr.of[Int], TypeRepr.of[String]), _ => TypeRepr.of[Int])
    val rt = Refinement(TypeRepr.of[AnyRef], "f", mt)
    val arg = Typed(Literal(NullConstant()), Inferred(rt))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a pattern binder's information: `case x @ _` whose binder's type is `Int @tagged(7)`
   *  (`first`) or `Int @tagged(8)` (the review's probe, out/codex-probes/type-representation-push-250632571/Make.scala). */
  transparent inline def bind(inline first: Boolean): Any = ${ bindImpl('first) }

  def bindImpl(first: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val tag = Apply(Select(New(TypeTree.of[tagged]), TypeRepr.of[tagged].typeSymbol.primaryConstructor),
      List(Literal(IntConstant(if first.valueOrAbort then 7 else 8))))
    val x = Symbol.newBind(Symbol.spliceOwner, "x", Flags.EmptyFlags, AnnotatedType(TypeRepr.of[Int], tag))
    val arg = Match(Literal(IntConstant(1)), List(CaseDef(Bind(x, Wildcard()), None, Literal(IntConstant(0)))))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a lambda's variances: an argument typed by a refinement holding `type F[+X]` (`co`) or
   *  `type F[-X]`, the two lambdas alike but for their parameter's variance (the review's probe,
   *  out/codex-probes/type-representation-push-9cab29947/Make.scala). */
  transparent inline def variance(inline co: Boolean): Any = ${ varianceImpl('co) }

  /** The detection pair of a named type's designator: `DDerived`'s selection of the base's private `Inner` (`base`)
   *  or of its own, one prefix and one name, two symbols (the review's probe on d54cf60be,
   *  out/codex-probes/type-representation-push-d54cf60be/designator/Make.scala). */
  transparent inline def designator(inline base: Boolean): Any = ${ designatorImpl('base) }

  def designatorImpl(base: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val owner = if base.valueOrAbort then TypeRepr.of[DBase].typeSymbol else TypeRepr.of[DDerived].typeSymbol
    val selected = TypeRepr.of[DDerived].select(owner.declaredType("Inner").head)
    val arg = Typed(Literal(NullConstant()), Inferred(selected))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a return's target: `Return(1, jump)` over the first or the second of `Pickled`'s two
   *  overloads of `jump` (the review's probe, out/codex-probes/type-representation-push-16b90a962/return-target). */
  transparent inline def returnTarget(inline first: Boolean): Any = ${ returnTargetImpl('first) }

  def returnTargetImpl(first: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val targets = Symbol.requiredModule("repr.rec.Pickled").moduleClass.methodMember("jump").sortBy(_.signature.toString)
    val arg = Return(Literal(IntConstant(1)), targets(if first.valueOrAbort then 0 else 1))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a target name: `Return(1, jumpL)` over the one of `Pickled`'s two overloads of `jumpL`, of
   *  one erasure, whose `@targetName` is `jumpInt` or `jumpString` (the review's probe, out/codex-probes/
   *  type-representation-push-c1be497fd/target-name). */
  transparent inline def returnTargetNamed(inline first: Boolean): Any = ${ returnTargetNamedImpl('first) }

  def returnTargetNamedImpl(first: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val targets = Symbol.requiredModule("repr.rec.Pickled").moduleClass.methodMember("jumpL")
    val tag = TypeRepr.of[scala.annotation.targetName].typeSymbol
    val wanted = if first.valueOrAbort then "jumpInt" else "jumpString"
    val target = targets.find(_.getAnnotation(tag).exists(_.show.contains(wanted))).get
    val arg = Return(Literal(IntConstant(1)), target)
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** A return to the method `method` of `Pickled` whose `@targetName` is `target` (the review's probe on 4c8ba8586:
   *  overloads of one erasure whose target names are `super$jumpS` and `jumpS`). */
  transparent inline def returnTo(inline method: String, inline target: String): Any = ${ returnToImpl('method, 'target) }

  def returnToImpl(method: Expr[String], target: Expr[String])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val tag = TypeRepr.of[scala.annotation.targetName].typeSymbol
    val wanted = "\"" + target.valueOrAbort + "\""
    val chosen = Symbol.requiredModule("repr.rec.Pickled").moduleClass.methodMember(method.valueOrAbort)
      .find(_.getAnnotation(tag).exists(_.show.contains(wanted))).get
    val arg = Return(Literal(IntConstant(1)), chosen)
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a float's raw bits: a `FloatConstant` holding a signalling NaN (`0x7f800001`) or its quiet
   *  twin (`0x7fc00001`), which no source writes and a macro pickles (the review's probe, out/codex-probes/
   *  type-representation-push-c1be497fd/float-nan). */
  transparent inline def floatBits(inline signalling: Boolean): Any = ${ floatBitsImpl('signalling) }

  def floatBitsImpl(signalling: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val raw = if signalling.valueOrAbort then 0x7f800001 else 0x7fc00001
    val arg = Literal(FloatConstant(java.lang.Float.intBitsToFloat(raw)))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of an access boundary: a local method `x` private within `Outer` (`outer`) or within
   *  `Outer.Product` (the review's probe, out/codex-probes/type-representation-push-3e054888f/Make.scala). */
  transparent inline def within(inline outer: Boolean): Any = ${ withinImpl('outer) }

  def withinImpl(outer: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val boundary = Symbol.requiredModule(if outer.valueOrAbort then "repr.rec.Outer" else "repr.rec.Outer.Product").moduleClass
    val x = Symbol.newMethod(Symbol.spliceOwner, "x", MethodType(Nil)(_ => Nil, _ => TypeRepr.of[Int]), Flags.EmptyFlags, boundary)
    val arg = Block(List(DefDef(x, _ => Some(Literal(IntConstant(1))))), Literal(IntConstant(0)))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

  /** The detection pair of a signed name's parameter list: a call to the first of `Overloads`' two `f` overloads, whose
   *  first parameter's type is a class whose name holds the signature separators (`first`), or to the second, whose
   *  first parameter is `X`; the two names flatten to one text (the review's pair,
   *  out/codex-probes/type-representation-push-b047da41b/name-collision). */
  transparent inline def signed(inline first: Boolean): Int = ${ signedImpl('first) }

  def signedImpl(first: Expr[Boolean])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val m = Symbol.requiredModule("repr.rec.Overloads")
    val target = m.moduleClass.methodMember("f").find { s =>
      s.signature.paramSigs.head.toString.contains(", ") == first.valueOrAbort
    }.getOrElse(report.errorAndAbort("no f overload with the requested first parameter signature"))
    Apply(Select(Ref(m), target), List(Literal(NullConstant()), Literal(NullConstant()))).asExprOf[Int]

  def varianceImpl(co: Expr[Boolean])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val base = if co.valueOrAbort then TypeRepr.of[Co] else TypeRepr.of[Contra]
    val bounds = base.memberType(base.typeSymbol.declaredType("F").head)
    val arg = Typed(Literal(NullConstant()), Inferred(Refinement(TypeRepr.of[Any], "F", bounds)))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

/** `Pickled.Sup`'s parent: a `super` selection in an annotation argument (`This`, `Super`, `SuperType`). */
class Base:
  def b: Int = 1

/** Two traits for an intersection (`AndType`). */
trait A1
trait B1

/** `Built.built`'s tree no source writes into an argument: a type block. */
object Built:
  transparent inline def built: Any = ${ builtImpl }

  def builtImpl(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val typed = Typed(Literal(IntConstant(1)), TypeBlock(Nil, TypeTree.of[Int]))
    val arg = Block(List(typed), Literal(IntConstant(0)))
    val annot = Apply(Select(New(TypeTree.of[kinds]), TypeRepr.of[kinds].typeSymbol.primaryConstructor), List(arg))
    AnnotatedType(TypeRepr.of[Int], annot).asType match
      case '[t] => '{ 1.asInstanceOf[t] }

/** `Kinds.designator`'s two declarations of `Inner`, one private to the base, under one `DDerived` prefix. */
class DBase { private class Inner }
class DDerived extends DBase { class Inner }

/** `Kinds.within`'s two access boundaries. */
object Outer:
  object Product

/** `Kinds.variance`'s two declarations of `F`. */
trait Co { type F[+X] }
trait Contra { type F[-X] }

object Lib:
  transparent inline def mk: Any = ${ mkImpl }

  def mkImpl(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val rt = RecursiveType(_ => Refinement(TypeRepr.of[AnyRef], "a", TypeRepr.of[Int]))
    rt.asType match
      case '[t] => '{ (null: Any).asInstanceOf[t] }
