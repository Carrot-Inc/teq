// expect: 18:11: error: Maximal number of successive inlines (32) exceeded
// expect: 21:19: error: Cannot reduce `inline if` because its condition is not a constant value: n > 0
// expect: 26:19: error: cannot reduce inline match with
// expect: patterns :  case 1
// expect: 29:11: error: Int is not a constant type; cannot take constValue
// expect: 32:11: error: No given instance of type Ordering[Shape] was found.
// expect: 35:11: error: unknown shape
// expect: 38:19: error: expected a constant value but found
// expect: 7 errors found
// The errors of inline expansions, the inlining phase's, each at its outermost call, in a program the
// typer reported none in (inline_errors has the typer's).
import scala.compiletime.{constValue, error, requireConst, summonInline}

trait Shape

object Errors:
  inline def forever(inline n: Int): Int = forever(n + 1)
  def a = forever(1)

  inline def sign(n: Int): Int = inline if n > 0 then 1 else -1
  def b(x: Int) = sign(x)

  inline def name(n: Int): String = inline n match
    case 1 => "one"
    case 2 => "two"
  def c(x: Int) = name(x)

  inline def width[T] = constValue[T]
  def g = width[Int]

  inline def order[T] = summonInline[Ordering[T]]
  def h = order[Shape]

  inline def refuse(inline what: String): Nothing = error("unknown " + what)
  def i = refuse("shape")

  inline def constant(inline n: Int): Unit = requireConst(n)
  def j(k: Int) = constant(k)
