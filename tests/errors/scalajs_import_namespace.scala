// teq: tests/interop/scalajs-stub
// The second argument of @JSImport is a literal string or the object JSImport.Namespace, by its symbol
// (PrepJSInterop.checkJSImportLiteral): the namespace import passes, a value that is neither is reported at the
// argument, a call whose result is the object's type too (an application's symbol is its function's), as scalac 3.8.4
// --js has it (L:C is where its caret begins, 1-based).
// expect: 23:24: error: The second argument to @JSImport must be literal string or the JSImport.Namespace object.
// expect: 28:24: error: The second argument to @JSImport must be literal string or the JSImport.Namespace object.
// expect: 2 errors found
package app

import scala.scalajs.js
import scala.scalajs.js.annotation.*

object Names:
  val computed: String = "jo".concat("in")
  def ns(): JSImport.Namespace.type = JSImport.Namespace

@js.native @JSImport("node:path", JSImport.Namespace)
object Path extends js.Object:
  def basename(path: String): String = js.native

@js.native
@JSImport("node:path", Names.computed)
object Bad extends js.Object:
  def basename(path: String): String = js.native

@js.native
@JSImport("node:path", Names.ns())
object Called extends js.Object:
  def basename(path: String): String = js.native
