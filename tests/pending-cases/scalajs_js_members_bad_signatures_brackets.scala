// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-members-bad-signatures.scala,
// lines 1-6 and 12-39: transformValOrDefDefInJSType's
// native @JSBracketAccess and @JSBracketCall arms
// (checkNoDefaultOrRepeated), its errors as scalac 3.8.4
// -scalajs gives them on this file. An expectation's L:C is where
// scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to report the eight.
// expect: 26:3: error: @JSBracketAccess methods must have one or two parameters
// expect: 29:3: error: @JSBracketAccess methods must have one or two parameters
// expect: 32:3: error: @JSBracketAccess methods with two parameters must return Unit
// expect: 35:3: error: @JSBracketAccess methods may not have repeated parameters
// expect: 38:3: error: @JSBracketAccess methods may not have default parameters
// expect: 43:3: error: @JSBracketCall methods must have at least one non-repeated parameter
// expect: 46:3: error: @JSBracketCall methods must have at least one non-repeated parameter
// expect: 49:3: error: @JSBracketCall methods must have at least one non-repeated parameter
// expect: 8 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
@JSGlobal
class A extends js.Object {
  // @JSBracketAccess

  @JSBracketAccess
  def b1(): Int = js.native // error

  @JSBracketAccess
  def b2(x: Int, y: Int, z: Int): Int = js.native // error

  @JSBracketAccess
  def b3(x: Int, y: Int): Int = js.native // error

  @JSBracketAccess
  def b4(x: Int*): Int = js.native // error

  @JSBracketAccess
  def b5(x: Int = 1): Int = js.native // error

  // @JSBracketCall

  @JSBracketCall
  def c1(): Int = js.native // error

  @JSBracketCall
  def c2(xs: String*): Int = js.native // error

  @JSBracketCall
  def c3(xs: String*)(y: String): Int = js.native // error
}
