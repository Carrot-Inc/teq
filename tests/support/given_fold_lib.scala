// The library tests/workers/library_given_summon and tests/support/library_value_reads read, compiled by scalac
// (tests/support/jars.sh's givenfold-lib): givens and values a client reads through an inline call.
package givenfold

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
