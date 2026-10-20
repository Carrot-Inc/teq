// `B1.x` reads `B2.y`, whose inferred type reads `A.inferred`, which a transparent macro of the run suspended: `B`
// suspends where its walk is, at `x` (`Namer.SuspendCompleter`, the completion nested in the walk), and `B15`, after
// it, is not typed in this run, which the errors end (`Driver.finish`): scalac reports `B1`'s error and `Z`'s.
// expect: B.scala:2:23: error: type mismatch: found 123, required String
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 2 errors found
// absent: B.scala:5:21
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
