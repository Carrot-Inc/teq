// `@nowarn` on an enclosing definition silences the warning.
import scala.annotation.nowarn
object Lib { val a = 1; val b = 2 }
object Use {
  @nowarn def f = { import Lib.a; 0 }
  def g = { import Lib.b; 0 }
}
