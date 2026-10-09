// A tuple index is static where scalac types it as a constant; a widened one, an ascription to
// `Int`, an inline `def` of a declared `Int`, a `val`, a plain `def` of a literal type, a
// `final val` of a declared `Int`, a block with an effect, is read at run time, where past the
// elements it throws, through an inline method's parameter too. A literal index past the
// elements in an inline body is the expansion's error: an `inline if` that drops the branch, or
// no call at all, never reports it. A static index in range keeps the element's type, through a
// val of a literal type, a parameter, a `constValue` and a tuple a type parameter stood for. An
// ascription widens whatever its operand comes to be: an inline or a plain parameter's
// (`((i: Int))`, an operation over one), a `final val`'s initialiser, an inline method's body,
// a by-name parameter's read, an `inline if`'s or `inline match`'s branch, a nested inline
// call, a chain of them; a transparent method's widened result keeps its type. An operation
// over an ascription folds (`final val k = (0: Int) + 0` is the constant 0), one over a plain
// inline call does not (`idx + 0`), nor under an ascription of it (`((idx + 0): Int) + 0`), nor
// over a block that ends in one.
import scala.compiletime.constValue
inline def idx: Int = 2
inline def zero: Int = 0
inline def choose(inline b: Boolean): Any =
  inline if b then (1, 2)(2) else 42
inline def unused: Any = (1, 2)(2)
def single: 2 = 2
object W:
  final val w: Int = 2
inline def pick(inline i: Int): Any = (1, 2)(i)
inline def plain(i: Int): Any = (1, 2)(i)
inline def generic[T <: Tuple](t: T): Any = t((2: Int))
transparent inline def second(inline i: Int) = (1, "b")(i)
transparent inline def nth[N <: Int] = (1, "c")(constValue[N])
transparent inline def first[T <: Tuple](t: T) = t(1)
transparent inline def widened(inline i: Int): Any = (1, "w")((i: Int))
transparent inline def widenedPlain(i: Int): Any = (1, 2)((i: Int))
transparent inline def widenedSum(inline i: Int): Any = (1, 2)(((i + 1): Int))
transparent inline def widen(inline i: Int) = (i: Int)
object F:
  final val two = (2: Int)
  final val zero = (0: Int)
def tag(x: Int): String = "Int"
def tag(x: Any): String = "Any"
transparent inline def byName(i: => Int) = (i: Int)
transparent inline def viaIf(inline i: Int) = ((inline if true then i else 1): Int)
transparent inline def viaMatch(inline i: Int) = ((inline i match { case 0 => 0; case _ => 1 }): Int)
transparent inline def same(inline i: Int) = i
transparent inline def viaCall(inline i: Int) = (same(i): Int)
transparent inline def chained(inline i: Int) = viaCall(i)
object G:
  final val folded = (0: Int) + 0
  final val through = same((0: Int))

def attempt(label: String)(read: => Any): Unit =
  try println(label + " " + read)
  catch case e: IndexOutOfBoundsException => println(label + " throws " + e.getClass.getName)

@main def run(): Unit =
  val n: Int = 2
  attempt("ascribed")((1, 2)((2: Int)))
  attempt("inline def")((1, 2)(idx))
  attempt("val")((1, 2)(n))
  attempt("in range")((1, "a")((1: Int)))
  attempt("def")((1, 2)(single))
  attempt("final val")((1, 2)(W.w))
  attempt("block")((1, 2)({ println("effect"); 2 }))
  attempt("inline parameter")(pick((2: Int)))
  attempt("plain parameter")(plain((2: Int)))
  attempt("parameter final val")(pick(W.w))
  attempt("type parameter")(generic((1, 2)))
  println(choose(false))
  println((1, 2, 3)(1 + 1))
  val one: 1 = 1
  val a: String = (1, "a")(one)
  val b: String = second(1)
  val c: String = nth[1]
  val d: String = first((1, "d"))
  println(a + b + c + d)
  println(tag(widened(0)) + " " + tag((1, "x")(F.zero)))
  attempt("ascribed parameter")(widened(2))
  attempt("ascribed plain parameter")(widenedPlain(2))
  attempt("ascribed sum")(widenedSum(1))
  attempt("ascribed final val")((1, 2)(F.two))
  attempt("ascribed body")((1, 2)(widen(2)))
  println(tag((1, "x")(byName(0))) + " " + tag((1, "x")(viaIf(0))) + " " + tag((1, "x")(viaMatch(0))) + " " + tag((1, "x")(viaCall(0))) + " " + tag((1, "x")(chained(0))))
  println(tag((1, "x")(G.folded)) + " " + tag((1, "x")(G.through)))
  attempt("by-name")((1, 2)(byName(2)))
  attempt("inline if")((1, 2)(viaIf(2)))
  attempt("inline call operation")((1, 2)(idx + 0))
  attempt("ascribed inline call operation")((1, 2)(((idx + 0): Int) + 0))
  attempt("block of inline call")((1, 2)(({ idx }) + 0))
  println(tag((1, "x")(((zero + 0): Int) + 0)) + " " + tag((1, "x")(({ zero }) + 0)) + " " + tag((1, "x")(({ val x = 1; zero }) + 0)))
