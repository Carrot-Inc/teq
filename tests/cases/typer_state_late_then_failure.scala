import scala.compiletime.{error, summonInline}

// A transparent given's expansion holds a `compiletime.error` (scalac's `Inlining` phase's, no
// failure of the candidate) and then a search that fails: the expansion goes on past the first and
// the second rejects the candidate, so the fallback is taken, as scalac takes it.
class Missing
class W(val n: Int)
trait Low:
  given fallback: W = new W(0)
object W extends Low:
  transparent inline given chosen: W =
    error("discarded candidate")
    summonInline[Missing]
    new W(7)
@main def main(): Unit = println(summon[W].n)
