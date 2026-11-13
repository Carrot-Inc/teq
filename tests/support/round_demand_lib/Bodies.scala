// A library whose bodies a macro's run reaches for the first time while the inlining phase expands a file that
// then waits for a macro of its run (tests/cases/macro_suspended_library_*): scalac compiles this directory
// into the jar rounddemand-lib, and each body is typed from its pickle when it is first asked for. What is
// typed then is the library's own, whatever becomes of the file whose expansion asked. A body whose own
// macro's call could not be expanded then keeps the call (tests/errors/unexpanded_library_call_*,
// tests/classpath/jvm/unexpanded_library_call_*).
package rounddemand

import scala.quoted.*

// A body whose inline call gives an erased result, unboxed where it is consumed.
object Erased:
  def blank[T]: T = null.asInstanceOf[T]
  inline def fetch[T]: T = blank[T]
  def whole: Int = fetch[Int]

// A macro whose first run calls that body.
object Warmed:
  private var cold: Boolean = true
  inline def one: Int = ${ oneImpl }
  def oneImpl(using Quotes): Expr[Int] =
    if cold then
      cold = false
      val warm: Int = Erased.whole
    Expr(1)

// A class whose template holds an inline call (an array's literal).
class Tally:
  private var seen = 0
  seen = Array(3, 1, 4).length
  def total: Int = seen

// A method whose default argument holds an inline call.
object Defaults:
  inline def two: Int = 2
  def sized(xs: Array[Int], upTo: Int = two): Int = xs.take(upTo).length

// An object a program may define as well, read by a macro of the library's.
object Depth:
  def measure: Int = 13

object Derived:
  inline def derived: Int = ${ derivedImpl }
  def derivedImpl(using Quotes): Expr[Int] = Expr(Depth.measure)

// A macro whose run takes the first branch of a body with two.
object Chosen:
  inline def one: Int = ${ oneImpl }
  def oneImpl(using Quotes): Expr[Int] = Expr(Reader.either(true))

// A macro whose first run alone calls a body that holds a call of the library's own macro.
object Once:
  private var cold: Boolean = true
  inline def one: Int = ${ oneImpl }
  def oneImpl(using Quotes): Expr[Int] =
    if cold then
      cold = false
      val reached: Int = Reader.depth
    Expr(1)

// A macro whose run takes the first branch of a body whose other branch calls the library's own macro.
object Settled:
  inline def one: Int = ${ oneImpl }
  def oneImpl(using Quotes): Expr[Int] = Expr(Reader.settled(true))

// An inline method that implements a member that is not inline: scalac keeps a method of its own for it,
// which a call left as a call links to.
trait Doubles:
  def twice(x: Int): Int

trait DoublesInline extends Doubles:
  override inline def twice(x: Int): Int = x + x

object Doubled extends DoublesInline

// An inline method that implements nothing: it has no method of its own.
object Tripled:
  inline def thrice(x: Int): Int = x + x + x

// A val and an object behind an export, whose forwarders a program names as paths.
object Stock:
  val plain: String = "p"
  object Inner:
    val n = 9

object Counter:
  println("counter-init")
  export Stock.{plain, Inner}
