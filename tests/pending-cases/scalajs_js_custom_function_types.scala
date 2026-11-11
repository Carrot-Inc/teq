//> using platform js
// dotty 3.8.4's tests/neg-scalajs/js-custom-function-types-prepjsinterop.scala (no .check; js.Function is std's): a
// JS type's apply where it is no JS function type (transformValOrDefDefInJSType), its errors as scalac 3.8.4 -scalajs
// gives them on this file. An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq's check phase reports RefChecks' "class BadFunctionIsClass needs to be abstract" (dotty's RefChecks
// runs after PrepJSInterop, not past its errors) and its own overload restriction, closing PrepJSInterop's gate: none
// of the seven is reported.
// expect: 20:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 23:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 27:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 30:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 33:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 34:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 37:3: error: A non-native JS type can only declare an abstract method named `apply` without `@JSName` if it is the SAM of a trait that extends js.Function
// expect: 7 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

class BadFunctionIsClass extends js.Function {
  def apply(x: Int): Int // error
}
trait BadFunctionExtendsNonFunction extends js.Object {
  def apply(x: Int): Int // error
}
class SubclassOfFunction extends js.Function
trait BadFunctionExtendsSubclassOfFunction extends SubclassOfFunction {
  def apply(x: Int): Int // error
}
trait BadFunctionParametricMethod extends js.Function {
  def apply[A](x: A): A // error
}
trait BadFunctionOverloaded extends js.Function {
  def apply(x: Int): Int // error
  def apply(x: String): String // error
}
trait BadFunctionMultipleAbstract extends js.Function {
  def apply(x: Int): Int // error
  def foo(x: Int): Int
}
