// expect: Maximal number of successive inlines (32) exceeded
// expect: Cannot reduce `inline if` because its condition is not a constant value: n > 0
// expect: cannot reduce inline match with
// expect: patterns :  case 1
// expect: inline value must contain a literal constant value.
// expect: inline value must have a literal constant type
// expect: inline if can only be used in an inline method
// expect: inline match can only be used in an inline method
// expect: No explicit return allowed from inlineable method early
// expect: Int is not a constant type; cannot take constValue
// expect: No given instance of type Ordering[Shape] was found.
// expect: unknown shape
// expect: expected a constant value but found
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
