// CanEqual on ==: resolveScoped credits a given or a named selector, never a plain wildcard.
object G {
  implicit val eqInt: CanEqual[Int, Int] = CanEqual.derived
}
import G.*
object Use { val x = 1 == 2 }

// teq: --werror --wunused imports
// expect: unused_imports_can_equal_wildcard.scala:5:10: warning: unused import
