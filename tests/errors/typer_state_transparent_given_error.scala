// expect: 15:33: error: candidate failed
// expect: 1 error found
// A transparent given's expansion holds the plain call of
// `compiletime.error`, which scalac expands after typing, so the candidate is the search's answer
// and the error is reported.
import scala.compiletime.{summonFrom, error}

transparent inline given bad: String = error("candidate failed")

transparent inline def choose: Int = summonFrom {
  case s: String => 0
  case _ => 42
}

@main def run(): Unit = println(choose)
