// teq: tests/interop/scalajs-stub
// Type tests of JS traits: scalac 3.8.4 --js rejects them in its back end (JSCodeGen), every earlier phase passing the
// program. An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// expect: 19:5: error: isInstanceOf[app.NativeTrait] not supported because it is a JS trait
// expect: 19:36: error: isInstanceOf[app.JsTrait] not supported because it is a JS trait
// expect: 2 errors found
package app

import scala.scalajs.js

@js.native
trait NativeTrait extends js.Object

trait JsTrait extends js.Object:
  val name: String

object Facade:
  def tests(x: Any): Boolean =
    x.isInstanceOf[NativeTrait] || x.isInstanceOf[JsTrait]
