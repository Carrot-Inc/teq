// A body typed on demand while a flush expands the inline call that reads it is a unit of its
// own (`state::as_own_unit`): the plain given `bad` found there for the argument of the transparent
// `preferred` fails in its own search, which takes `okInt`, as scalac and master print `7`.
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

object Main:
  @main def run(): Unit = println(go)
  inline def go: Int = helper.n
  def helper = summon[R]
