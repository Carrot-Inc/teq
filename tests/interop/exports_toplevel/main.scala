// @JSExportTopLevel on members of an object: the exports read the object, whose body runs when
// the module loads, as the application keeps its CSS imports alive. Compiled with
// tests/interop/scalajs-stub.
package app

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSExportTopLevel, JSImport}

@JSExportTopLevel("version")
val version: String = "1.0"

object Main:
  @js.native @JSImport("node:os", JSImport.Default)
  private object OsModule extends js.Object

  @JSExportTopLevel("osModule")
  val osModule: js.Object = OsModule

  @JSExportTopLevel("twice")
  def twice(x: Int): Int = x * 2

  val counter: Int =
    println("Main initialised")
    1
