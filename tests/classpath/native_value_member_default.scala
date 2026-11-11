// jars: scala-library scalajs-library
package lib:
  import scala.scalajs.js
  import scala.scalajs.js.annotation.*

  @js.native @JSGlobal
  class Screen extends js.Object:
    def measure(el: String, mode: String = js.native): String = js.native

  object host:
    @js.native @JSGlobal("screen")
    val global: Screen = js.native

    @js.native @JSImport("screens", "main")
    val imported: Screen = js.native

package use:
  import lib.host.global.measure
  import lib.host.imported.measure as measureImported

  object Use:
    def one: String = measure("div")
    def two: String = measure("div", "hover")
    def three: String =
      import lib.host.global.*
      measure("span")
    def four: String = measureImported("div")
    def five: String = lib.host.global.measure("div") + lib.host.imported.measure("div")
