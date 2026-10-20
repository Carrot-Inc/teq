// `B` suspends at `A.inferred`, and `B.z`, after it, is not typed by `B`'s walk: dotty's `SuspendException` leaves
// the unit's typer at once, and `B.z` stays a lazy completer, which `C`'s read completes, its error reported and
// standing. `C` does not suspend; `Z`'s error and `B.z`'s end the runs: scalac reports both. (Before, the walk went
// on over `B` unwinding, published `B.z` suspended, and `C` suspended on it, the error lost at one worker.)
// expect: B.scala:3:29: error: type mismatch: found 1, required String
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 2 errors found
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
