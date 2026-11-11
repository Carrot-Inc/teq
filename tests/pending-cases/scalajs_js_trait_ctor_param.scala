// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-trait-ctor-param.scala: a non-native JS trait's constructor parameter
// (transformValOrDefDefInJSType), its errors as scalac 3.8.4 -scalajs gives them on this file. An expectation's L:C
// is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: the stub's js.Any is @js.native (scalajs-library 1.22.0's is not), so teq adds "Non-native JS types cannot
// directly extend native JS traits."; and teq gives the parameter's error at its name (column 35) where scalac's
// caret begins at its `val` (column 31).
// expect: 18:31: error: A non-native JS trait cannot have constructor parameters
// expect: 1 error found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
trait Bag extends js.Any {
  val str: String
}

trait NonNativeBagHolderTrait(val bag: Bag) extends js.Any // error
