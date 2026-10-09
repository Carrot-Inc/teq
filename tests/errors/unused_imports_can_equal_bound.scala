// CanEqual on ==: a given selector is credited only with the givens conforming to its type.
package canequal_bound
object G {
  given ii: CanEqual[Int, Int] = CanEqual.derived
  given ss: CanEqual[String, String] = CanEqual.derived
}
import G.given CanEqual[String, String]
object Use { val x = 1 == 2 }

// teq: --werror --wunused imports
// expect: unused_imports_can_equal_bound.scala:7:10: warning: unused import
