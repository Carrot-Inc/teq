// The arguments of a transparent given the search expands where it finds it are resolved with the
// plain inline givens among them expanded where they are found: `bad` fails in its own search for
// an `Int`, which takes `okInt`, and `preferred` is the instance, not the lower `fallback`, as
// scalac (which drops the unused argument) and master print.
class R(val n: Int)
trait Low:
  given fallback: R = R(1)
object R extends Low:
  transparent inline given preferred(using inline x: Int): R = R(7)

trait LowInt:
  given okInt: Int = 5
object Ints extends LowInt:
  inline given bad: Int = scala.compiletime.error("bad failed")
import Ints.given

@main def run(): Unit = println(summon[R].n)
