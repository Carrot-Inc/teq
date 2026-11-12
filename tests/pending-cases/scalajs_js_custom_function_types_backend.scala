//> using platform js
// dotty 3.8.4's tests/neg-scalajs/js-custom-function-types-backend.scala (no .check): a
// js.ThisFunction's SAM without a leading non-varargs parameter (JSCodeGen#genClosure's rejection,
// an error of the back end), its errors as scalac 3.8.4 -scalajs gives them on this file.
// An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// pending on SAMType and the switch: teq is to type both lambdas and report the two.
// expect: 20:44: error: The SAM or apply method for a js.ThisFunction must have a leading non-varargs parameter
// expect: 21:44: error: The SAM or apply method for a js.ThisFunction must have a leading non-varargs parameter
// expect: 2 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

trait BadThisFunction1 extends js.ThisFunction {
  def apply(): Int
}
trait BadThisFunction2 extends js.ThisFunction {
  def apply(args: Int*): Int
}
class A {
  val badThisFunction1: BadThisFunction1 = () => 42 // error
  val badThisFunction2: BadThisFunction2 = args => args.size // error
}
