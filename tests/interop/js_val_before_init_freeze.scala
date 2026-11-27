// A Scala class an exported function makes, read from JavaScript (js_val_before_init_freeze.harness.mjs): whether its
// prototype holds the field's name, and the next instance's field after JavaScript freezes that prototype.
import scala.scalajs.js.annotation.JSExportTopLevel
class A { val f: Int = 7 }
object Factory {
  @JSExportTopLevel("create") def make(): A = new A
}
