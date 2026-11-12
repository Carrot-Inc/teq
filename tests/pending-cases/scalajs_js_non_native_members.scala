// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-non-native-members.scala: a non-native
// JS class's members named apply and its @JSBracketAccess/@JSBracketCall
// members (transformValOrDefDefInJSType), its errors as scalac
// 3.8.4 -scalajs gives them on this file. An expectation's L:C is
// where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to report the six.
// expect: 19:3: error: A non-native JS class cannot declare a concrete method named `apply` without `@JSName`
// expect: 21:3: error: @JSBracketAccess is not allowed in non-native JS classes
// expect: 24:3: error: @JSBracketCall is not allowed in non-native JS classes
// expect: 29:7: error: A member named apply represents function application in JavaScript. A parameterless member should be exported as a property. You must add @JSName("apply")
// expect: 38:7: error: A member named apply represents function application in JavaScript. A parameterless member should be exported as a property. You must add @JSName("apply")
// expect: 47:7: error: A member named apply represents function application in JavaScript. A parameterless member should be exported as a property. You must add @JSName("apply")
// expect: 6 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

class A extends js.Object {
  def apply(arg: Int): Int = arg // error

  @JSBracketAccess
  def foo(index: Int, arg: Int): Int = arg // error

  @JSBracketCall
  def foo(m: String, arg: Int): Int = arg // error
}

class B extends js.Object {
  def apply: Int = 42 // error
}

class BOK extends js.Object {
  @JSName("apply")
  def apply: Int = 42
}

class C extends js.Object {
  val apply: Int = 42 // error
}

class COK extends js.Object {
  @JSName("apply")
  val apply: Int = 42
}

class D extends js.Object {
  var apply: Int = 42 // error
}

class DOK extends js.Object {
  @JSName("apply")
  var apply: Int = 42
}
