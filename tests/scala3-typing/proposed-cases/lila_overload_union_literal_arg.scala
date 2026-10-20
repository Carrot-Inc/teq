// lila: round/RoundSocket.scala:303 (`disconnectTimeout * { speed match ... => 10 ... }`)
// An overloaded call whose argument is an if/match of Int literals (type 10 | 4) resolves with
// the argument widened to Int, so numeric widening to Long applies.
import scala.concurrent.duration.*

def timeout(classical: Boolean): FiniteDuration =
  30.seconds * {
    if classical then 10
    else 4
  }

@main def run() =
  println(timeout(true))
  println(timeout(false))
