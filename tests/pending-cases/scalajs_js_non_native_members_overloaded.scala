// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-non-native-members-overloaded.scala: private overloads in non-native JS classes
// (transformValOrDefDefInJSType), its errors as scalac 3.8.4 -scalajs gives them on this file. An expectation's L:C
// is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq's own overload restriction ("method foo is overloaded in class A, a JavaScript class ...") is a
// typer-phase error that closes PrepJSInterop's gate: none of dotty's six private-overload errors is reported.
// expect: 18:3: error: Private methods in non-native JS classes cannot be overloaded. Use different names instead.
// expect: 19:3: error: Private methods in non-native JS classes cannot be overloaded. Use different names instead.
// expect: 24:5: error: Private methods in non-native JS classes cannot be overloaded. Use different names instead.
// expect: 25:5: error: Private methods in non-native JS classes cannot be overloaded. Use different names instead.
// expect: 30:3: error: Private methods in non-native JS classes cannot be overloaded. Use different names instead.
// expect: 36:5: error: Private methods in non-native JS classes cannot be overloaded. Use different names instead.
// expect: 6 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

class A extends js.Object {
  private def foo(i: Int): Int = i // error
  private def foo(s: String): String = s // error
}

object BEnclosing {
  class B extends js.Object {
    private[BEnclosing] def foo(i: Int): Int = i // error
    private def foo(s: String): String = s // error
  }
}

class C extends js.Object {
  private def foo(i: Int): Int = i // error
  def foo(s: String): String = s
}

object DEnclosing {
  class D extends js.Object {
    private[DEnclosing] def foo(i: Int): Int = i // error
    def foo(s: String): String = s
  }
}
