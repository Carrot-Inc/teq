// expect: 14:61: error: selected-given-error
// expect: 1 error found
// A plain inline given that a lexical extension's prefix resolves is an instance: its expansion
// runs in the later phase, for the candidate selected, and its error is reported there, not a
// reason to take the companion's extension (scalac reports `selected-given-error` at the call,
// as teq does).
import scala.compiletime.error
class R
object R:
  extension (r: R) def pick: Int = 2
object Main:
  inline given bad: Int = error("selected-given-error")
  extension (r: R)(using Int) def pick: Int = 1
  def main(args: Array[String]): Unit = println((new R).pick)
