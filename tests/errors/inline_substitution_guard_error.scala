// expect: 10:33: error: guard failed
// expect: 1 error found
// An error of a guard's expansion stops the reduction of an `inline match` with its own
// message, as scalac reports it, rather than being taken for a case that does not match.
import scala.compiletime.error
inline def f(inline n: Int): Int =
  inline n match
    case 1 if error("guard failed") => 1
    case _ => 2
@main def run(): Unit = println(f(1))
