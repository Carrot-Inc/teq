// expect: 22:43: error: candidate failed
// expect: 1 error found
// A transparent given whose expansion holds a `compiletime.error` is the search's answer (the
// error is scalac's `Inlining` phase's, no failure of the candidate), also where the search sets
// it aside to try another candidate and takes it back: `R.other` fails, `R.bad` stands, and the
// error is reported, not a reason for `chosen` to give way to `fallback`.
import scala.compiletime.{error, summonInline}
given ready: Unit = ()
class Missing
class R
object R:
  transparent inline given bad(using Unit): R =
    error("candidate failed")
  given other(using Missing): R = new R
class W(val n: Int)
trait Low:
  given fallback: W = new W(0)
object W extends Low:
  transparent inline given chosen: W =
    summonInline[R]
    new W(7)
@main def main(): Unit = println(summon[W].n)
