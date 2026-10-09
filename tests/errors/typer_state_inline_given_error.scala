// expect: 17:25: error: candidate failed
// expect: 1 error found
// A plain inline given whose body is an error is the search's answer, its expansion's error
// reported after typing, where scalac reports it; not a reason to take the
// inherited fallback.
import scala.compiletime.error

trait Low:
  given fallback: String = "fallback"

object Givens extends Low:
  inline given bad: String = error("candidate failed")

import Givens.given

@main def run(): Unit =
  println(summon[String])
