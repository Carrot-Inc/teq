// A transparent given providing an extension, whose argument's plain inline given `bad` fails in
// its own search, which takes `okInt`: `hello`, as scalac and master print. The failed candidate's
// registration goes with its attempt, though a unit's flush under an enclosing attempt took the
// entries the attempt's position stood past (`state::pending_cut_at`).
class A
trait Ops:
  extension (a: A) def hello: String = "hello"
object A:
  transparent inline given ops(using inline x: Int): Ops = new Ops {}

trait LowInt:
  given okInt: Int = 5
object Ints extends LowInt:
  inline given bad: Int = scala.compiletime.error("bad failed")
import Ints.given

@main def run(): Unit = println(A().hello)
