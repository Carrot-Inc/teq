// The representation oracle's fixture (tests/tasty/representation/README.md): one member per shape a pickle has to
// carry, observed by the other compiler's reflection. check.sh makes the `@mark(8)` variant from this file.
package repr

class mark(val n: Int) extends scala.annotation.StaticAnnotation
class named(val a: Int, val b: String = "b") extends scala.annotation.StaticAnnotation
class tag(val x: Any) extends scala.annotation.StaticAnnotation
class memberTag(val n: Int) extends scala.annotation.StaticAnnotation
trait Parent[A]
// A secondary constructor's parameter annotation and a case class's synthetic `apply` (the review's probes on
// d54cf60be): the variant reads `@memberTag(58)` and `@memberTag(48)`.
class Ctors():
  def this(@memberTag(57) x: Int) = this()
case class Cased(x: Int @memberTag(47))
// A member named `<owner>`, which keyed the owner's own record before the fold of ff57bc432 (the review's probe), in a
// class and in its companion, both requested: the variant reads `@memberTag(68)`, seen through the generated traversal.
class Reserved:
  def `<owner>`: Int = 2
object Reserved:
  @memberTag(67) def `<owner>`: Int = 1
// An annotation inside an ordinary method's body (the review's probe on 1c61410d5), which the observation reaches only
// with the body retained (`-Yretain-trees`; without it `Symbol.tree` gives the method no body): the variant reads
// `@memberTag(78)`.
object Product:
  def f: Int =
    val x: Int @memberTag(77) = 1
    x
  // An annotation inside an initializer's statement (the review's probe on 92c156369), which the owner's body reaches
  // only as the template's statements: the variant reads `@memberTag(88)`.
  locally {
    val y: Int @memberTag(87) = 1
    println(y)
  }
// Names and strings holding an outcome field's marks as program text (the reviews' probe on 7a9ec6133 and febb521c4):
// no child of it is unread, and no check takes one for it.
class Marker:
  def `Threw(`: Int = 1
  val text = "Threw("
  def ` |Threw(`: Int = 2
  val field = " |Threw([Unread([x#0], rhs, MatchError, \"129\")])"
// A requested owner's own annotation (the review's probe on 87ecb26c3): the variant reads `@memberTag(38)`.
@memberTag(37) class Tagged:
  def x: Int = 1
class Text(val a: String, val b: String) extends scala.annotation.StaticAnnotation
class leaves(val b: Boolean, val by: Byte, val s: Short, val i: Int, val l: Long, val f: Float, val d: Double,
  val c: Char, val str: String, val cls: Class[?], val u: Any) extends scala.annotation.StaticAnnotation
class Annot[T] extends scala.annotation.Annotation

trait Box:
  type T
  val value: T

trait Abstract:
  type T <: String
object AObj extends Abstract

trait Bounded:
  type B >: Int <: Int
  type C <: AnyVal

class Outer:
  class Inner

class Holder(val n: Int)
// The encoding record's paths (the coordinator's check of f719a99f0b): a definition literally named `a/b` and a `b`
// inside an `a`, each with the same method and body, so a line that joins the path's names with `/` gives their
// children's lines one value.
object Paths:
  class `a/b`:
    def pair(x: Int): Int = x
  class a:
    class b:
      def pair(x: Int): Int = x
object Values:
  val a = Holder(7)
  val b = Holder(8)
  val flag: Boolean = false

trait Gen:
  def g[T](x: T): Any

// dotty's tests/pos/annot-5789.scala, whose acceptance proves nothing of the annotation's preservation.
class D[T](val f: Int @Annot[T])

object API:
  // References to aliases, kept until a reader expands them.
  type A = Int
  def alias(x: A): A = x
  type F[X] = List[X]
  def applied: F[Int] = Nil
  type AA = A
  def aliasOfAlias: AA = 1
  type U = 1 | 2
  def u: U = 1
  type M[X] = X match
    case Int => String
  def matched: M[Int] = ""

  // Abstract and opaque references.
  def abs: AObj.T = ???
  opaque type O = Int
  def opq: O = 1

  // Annotations and their arguments.
  def ann(x: Int @mark(7)): Int @mark(7) = x
  def annList: List[Int @mark(7)] = Nil
  def annNamed: Int @named(1) = 1
  // An argument's receiver: check.sh's variant reads `Values.b.n` where this has `Values.a.n`.
  def annReceiver: Int @mark(Values.a.n) = 1
  // An argument's ascription: the variant reads `(null: Object)` where this has `(null: String)`.
  def annTyped: Int @tag((null: String)) = 1
  // Two string arguments whose unquoted texts would meet: the variant reads the other split.
  def annText: Int @Text("left), String(middle", "right") = 1
  // A root member's own annotation (the review's probe on fe56e6bcb): the variant reads `@memberTag(8)`.
  @memberTag(7) def taggedMember(x: Int): Int = x
  // A root method's parameter annotation and a root class's parent (the review's probes on 87ecb26c3): the variant
  // reads `@memberTag(18)` and `@memberTag(28)`.
  def paramTagged(@memberTag(17) x: Int): Int = x
  class Child extends Parent[Int @memberTag(27)]
  // A conditional's three children, each kept (rec/'s `kinds` pair tells its constants' kinds apart).
  def annIf: Int @tag(if Values.flag then 1 else 2) = 1
  // A line separator in a string: the variant reads U+2028 where this has U+0085, each escaped through every cell.
  def annSeparator: Int @tag("left\u0085right") = 1
  // A string holding a bounds mark's text: the variant reads `""`, and the T-S normalization keeps the string whole.
  def annMarkText: Int @tag("{TypeAlias}") = 1
  // Every character Python's `splitlines` takes for a line boundary, each escaped (README.md, "Transport").
  def annBoundaries: Int @tag("a\nb\rc\u000bd\u000ce\u001cf\u001dg\u001eh\u0085i\u2028j\u2029k") = 1
  // One argument of every constant kind, a string with a space, quotes, a newline and a backslash.
  def annLeaves: Int @leaves(true, 1, 2, 3, 4L, 5.5f, 6.5d, 'c', "a \"q\"\n\\b", classOf[String], ()) = 1
  // The constant kinds a final val's type keeps (Byte, Short and Unit widen there; annLeaves has them).
  final val leafBoolean = true
  final val leafInt = 3
  final val leafLong = 4L
  final val leafFloat = 5.5f
  final val leafDouble = -0.0d
  final val leafChar = '"'
  final val leafString = "a \"q\"\n\\b"
  final val leafClass = classOf[String]
  // A name holding delimiters.
  def `odd name, (x)`: Int = 1

  // Wildcard arguments and bounds.
  def bounds(x: List[? >: Int <: Int]): List[? >: Int <: Int] = x
  def upper: List[? <: AnyVal] = Nil
  def wild: List[?] = Nil
  def refinedAlias: Box { type T = Int } = ???
  def refinedBounds: Box { type T >: Int <: Int } = ???

  // Prefixes and designators.
  def dep(b: Box): b.T = b.value
  val box: Box = null
  def path: box.T = box.value
  def proj: Outer#Inner = ???
  def nested(o: Outer)(i: o.Inner): o.Inner = i
  def self: API.this.type = this

  // Binders and the references to them.
  def refinedMethod: Box { def f(x: Int): x.type } = ???
  def refinedPoly: Gen { def g[T](x: T): T } = ???
  def recursive: Box { type S; def h: S } = ???
  def polyFun: [X] => X => X = [X] => (x: X) => x
  type L = [X] =>> List[X]
  def lambdaApplied: L[Int] = Nil
  def depFun: (b: Box) => b.T = (b: Box) => b.value
  def byName(x: => Int): Int = x
  def implicitClause(implicit x: Int): Int = x
  def usingClause(using x: Int): Int = x
  def nullary: Int = 1
