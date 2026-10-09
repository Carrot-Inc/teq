// Inside a transparent method's expansion a plain inline given found for an argument of a
// transparent given fails in its own search, late or not, as on master: `okInt` is taken and
// `preferred` is the instance, `7`, as scalac (which drops the unused argument) prints.
import scala.compiletime.*

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

transparent inline def viaFromT: Int = summonFrom { case r: R => r.n }
@main def run(): Unit = println(viaFromT)
