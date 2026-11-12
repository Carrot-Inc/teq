// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-trait-members.scala: a non-native JS trait's concrete members
// (transformValOrDefDefInJSType), its errors as scalac 3.8.4 -scalajs gives them on this file. An expectation's L:C
// is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to give the six right-hand-side errors where scalac's caret begins, at
// the definition (column 3), and not that error at the two defs with parentheses, where
// dotty's UniqueMessagePositions hides it (its span covers the earlier error's offset).
// expect: 21:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 22:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 24:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 25:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 27:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 28:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 30:25: error: In non-native JS traits, defs with parentheses must be abstract.
// expect: 31:24: error: In non-native JS traits, defs with parentheses must be abstract.
// expect: 8 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

trait A extends js.Object {
  val a1: js.UndefOr[Int] = 5 // error
  val a2: Int = 5 // error

  def b1: js.UndefOr[Int] = 5 // error
  def b2: Int = 5 // error

  var c1: js.UndefOr[Int] = 5 // error
  var c2: Int = 5 // error

  def d1(x: Int): Int = x + 1 // error
  def d2[A](x: A): A = x // error
}
