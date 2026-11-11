// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-members-bad-signatures.scala, lines 1-10 and 39: PrepJSInterop's
// checkSetterSignature on a native JS class's setters (transformValOrDefDefInJSType), its errors as scalac 3.8.4
// -scalajs gives them on this file. An expectation's L:C is where scalac's caret begins (the start of dotty's span),
// 1-based.
// pending: teq gives dotty's four texts and the count, each at the setter's name (column 7) where scalac's caret
// begins at the definition (column 3): the walk's MemberDef#pos is the record's span, not the tree's.
// expect: 19:3: error: JS setters must return Unit
// expect: 20:3: error: JS setters must have exactly one argument
// expect: 21:3: error: JS setters may not have repeated params
// expect: 22:3: error: JS setters may not have default params
// expect: 4 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
@JSGlobal
class A extends js.Object {
  def a1_=(x: Int): Int = js.native // error
  def a2_=(x: Int, y: Int): Unit = js.native // error
  def a3_=(x: Int*): Unit = js.native // error
  def a4_=(x: Int = 1): Unit = js.native // error
}
