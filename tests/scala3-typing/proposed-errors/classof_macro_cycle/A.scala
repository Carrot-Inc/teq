// A macro whose run resolves a class of the run at run time (`classOf[H]`), H's file calling the macro: the class is
// missing from dotty's macro class loader in every run (`Interpreter.stopIfRuntimeException`, `suspendOnMissing`),
// so H.scala suspends in each and `Run.checkSuspendedUnits` reports the cycle. teq resolves `classOf` without the
// run's readiness check (`TExpr::ClassOf`, src/interp/eval.rs) and accepts the program, printing H, on master too.
import scala.quoted.*
object A:
  inline def value: String = ${ impl }
  def impl(using Quotes): Expr[String] = Expr(classOf[H].getName)
