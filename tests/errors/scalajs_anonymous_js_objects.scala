// teq: tests/interop/scalajs-stub
// Anonymous objects of a non-native JS trait: scalac 3.8.4 --js types them and PrepJSInterop passes them, and
// RefChecks rejects them. An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// expect: 19:5: error: object creation impossible, since val name: String in trait JsTrait in package app is not defined
// expect: 20:5: error: object creation impossible, since val name: String in trait JsTrait in package app is not defined
// absent: 20:23: error: value other is not a member of JsTrait
// expect: 21:5: error: object creation impossible, since val name: String in trait JsTrait in package app is not defined
// absent: 21:23: error: m cannot be defined in an anonymous JS object, which holds vals and assignments to the vars of its trait; write a class for methods
// expect: 3 errors found
package app

import scala.scalajs.js

trait JsTrait extends js.Object:
  val name: String

object Facade:
  def anonymous(): Unit =
    new JsTrait {}
    new JsTrait { val other = 1 }
    new JsTrait { def m = 1 }
