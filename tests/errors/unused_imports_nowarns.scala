// `@nowarn` on an enclosing definition silences the warning.
import scala.annotation.nowarn
object Lib { val a = 1; val b = 2 }
object Use {
  @nowarn def f = { import Lib.a; 0 }
  def g = { import Lib.b; 0 }
}

// teq: --werror --wunused imports
// expect: unused_imports_nowarns.scala:6:24: warning: unused import
