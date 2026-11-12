// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-trait-members-wrong-kind.scala: a non-native JS trait's lazy val, defs with
// parentheses and setter (transformValOrDefDefInJSType), its errors as scalac 3.8.4 -scalajs gives them on this file.
// An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to give the lazy val's error where
// scalac's caret begins, at the definition (column 3).
// expect: 16:3: error: A non-native JS trait cannot contain lazy vals
// expect: 18:30: error: In non-native JS traits, defs with parentheses must be abstract.
// expect: 19:36: error: In non-native JS traits, defs with parentheses must be abstract.
// expect: 20:27: error: In non-native JS traits, defs with parentheses must be abstract.
// expect: 4 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

trait A extends js.Object {
  lazy val a1: js.UndefOr[Int] = js.undefined // error

  def a(): js.UndefOr[Int] = js.undefined // error
  def b(x: Int): js.UndefOr[Int] = js.undefined // error
  def c_=(v: Int): Unit = js.undefined // error
}
