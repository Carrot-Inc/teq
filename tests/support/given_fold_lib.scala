// The library tests/workers/library_given_summon, tests/support/library_value_reads and
// tests/support/library_value_names read, compiled by scalac (tests/support/jars.sh's givenfold-lib): givens and
// values a client reads through an inline call, and an inline method whose default argument another one leaves out.
package givenfold

import scala.compiletime.erasedValue

object Givens:
  def three: Int = 3
  given Int = 11
  given Long = three.toLong
  given Boolean = three > 2
  given String = "s" + "t"

class C
object C:
  given Char = 'k'

object Outer:
  object Inner:
    given Double = 1.5

object Values:
  def three: Int = 3
  val x: Int = 11
  lazy val y: Int = three * 7
  final val k = 5

// `size` calls `walk` without its argument, and `walk` calls itself on `idx + 1` under an inline match: a client's
// expansion of `size` reads the default through the getter `walk$default$1` that scalac's body of `size` calls.
object Walk:
  inline def walk[T <: Tuple](idx: Int = 0): Int = inline erasedValue[T] match
    case _: EmptyTuple => idx
    case _: (h *: t) => walk[t](idx + 1)
  inline def size[T <: Tuple]: Int = walk[T]()
