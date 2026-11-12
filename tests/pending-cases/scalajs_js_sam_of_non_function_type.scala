//> using platform js
// dotty 3.8.4's tests/neg-scalajs/js-sam-of-non-function-type.scala (no .check; js.Function is std's):
// transformStatOrExpr's Closure arm, a lambda typed against a JS type that is no non-native JS function trait, its
// errors as scalac 3.8.4 -scalajs gives them on this file. An expectation's L:C is where scalac's caret begins (the
// start of dotty's span), 1-based.
// pending: teq is to type the
// lambdas against a JS SAM type
// (SAMType) and report the six.
// expect: 30:17: error: Using an anonymous function as a SAM for the JavaScript type A1 is not allowed because it is not a trait extending js.Function. Use an anonymous class instead.
// expect: 31:17: error: Using an anonymous function as a SAM for the JavaScript type A2 is not allowed because it is not a trait extending js.Function. Use an anonymous class instead.
// expect: 32:20: error: Using an anonymous function as a SAM for the JavaScript type A3 is not allowed because it is not a trait extending js.Function. Use an anonymous class instead.
// expect: 44:17: error: Using an anonymous function as a SAM for the JavaScript type B1 is not allowed because it is a native JS type. It is not possible to directly implement it.
// expect: 45:17: error: Using an anonymous function as a SAM for the JavaScript type B2 is not allowed because it is a native JS type. It is not possible to directly implement it.
// expect: 52:17: error: Using an anonymous function as a SAM for the JavaScript type C1 is not allowed because its single abstract method is not named `apply`. Use an anonymous class instead.
// expect: 6 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
trait A1 extends js.Object {
  def foo(x: Int): Int
}
trait A2 extends js.Object {
  def bar(x: Int): Int
}
class A3 extends js.Function {
  def foobar(x: Int): Int
}
class A4 {
  val foo: A1 = x => x + 1 // error
  val bar: A2 = x => x + 1 // error
  val foobar: A3 = x => x + 1 // error
}

@js.native
trait B1 extends js.Function {
  def apply(x: Int): Int
}
@js.native
trait B2 extends js.Function {
  def bar(x: Int = 5): Int
}
class B3 {
  val foo: B1 = x => x + 1 // error
  val bar: B2 = x => x + 1 // error
}

trait C1 extends js.Function {
  def foo(x: Int): Int
}
class C2 {
  val foo: C1 = x => x + 1 // error
}
