// expect: inline if can only be used in an inline method
// expect: inline match can only be used in an inline method
// expect: No explicit return allowed from inlineable method early
// expect: 3 errors found
// absent: Maximal number of successive inlines
// absent: Cannot reduce `inline if`
// absent: cannot reduce inline match with
// absent: inline value must
// absent: cannot take constValue
// absent: No given instance of type Ordering[Shape]
// absent: unknown shape
// absent: expected a constant value but found
// The typer's errors alone, as scalac's: the expansions' are the inlining phase's (inline_errors_expansion)
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

  inline val list = List(1)
  inline val typed: Int = { println(); 1 }

  def d(x: Int) = inline if x > 0 then 1 else 2
  def e(x: Int) = inline x match { case 1 => 2 }

  inline def early(x: Int): Int = { if x > 0 then return 1; 2 }
  def f = early(3)

  inline def width[T] = constValue[T]
  def g = width[Int]

  inline def order[T] = summonInline[Ordering[T]]
  def h = order[Shape]

  inline def refuse(inline what: String): Nothing = error("unknown " + what)
  def i = refuse("shape")

  inline def constant(inline n: Int): Unit = requireConst(n)
  def j(k: Int) = constant(k)
