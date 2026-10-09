// expect: 15:53: error: bad argument
// expect: 1 error found
// A plain inline call in an argument cached across an extension's alternatives: the transparent
// alternative expands it ahead of the later phase, and when that alternative is given up the call
// is pending again in the cached argument, so the error is the chosen alternative's, as scalac's.
import scala.compiletime.error
object M:
  inline def bad: Int = error("bad argument")
trait Ops:
  extension [T](x: Int)
    transparent inline def combine(inline y: String): String = "s" + y
  extension [T](x: Int)
    def combine(y: Int): String = "good"
object O extends Ops
@main def main(): Unit = println(O.combine[Unit](1)(M.bad))
