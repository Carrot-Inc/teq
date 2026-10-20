// `B` reports its error, then meets a transparent macro of the run: an error is reported, so `expandMacro`'s check is
// skipped, and the macro's run suspends `B` on the run's definition it reaches (`suspendOnMissing`); `B`'s later code
// is not typed in that run, and the errors prevent a later one: scalac reports `B`'s first error and `Z`'s.
// expect: B.scala:2:23: error: type mismatch: found 123, required String
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 2 errors found
// absent: B.scala:4:23
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
