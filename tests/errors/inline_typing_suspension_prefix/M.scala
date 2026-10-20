// `B` reports its error, then reads `A.inferred`, which a transparent macro of the run suspended: `B` suspends with
// it (`Namer.SuspendCompleter`), what it reported before stands (`TyperPhase.typeCheck` retracts nothing), and its
// later code waits for a later run, which the error prevents (`Driver.finish`): scalac reports the first error alone.
// expect: B.scala:2:23: error: type mismatch: found 123, required String
// expect: 1 error found
// absent: B.scala:4:23
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
