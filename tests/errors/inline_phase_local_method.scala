// expect: 11:21: error: type mismatch
// expect: 1 error found
// absent: a local expansion
// A local inline method's call is kept for the inlining phase as a member's is: its expansion's
// `error` is reported only after a typer that reported none, as scalac's `Inlining` runs.
import scala.compiletime.error
object O:
  def f: Int =
    inline def boom: Int = error("a local expansion")
    boom
  val bad: String = 1
