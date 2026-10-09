// CanEqual on ==: resolveScoped credits a given or a named selector, never a plain wildcard.
object G {
  implicit val eqInt: CanEqual[Int, Int] = CanEqual.derived
}
import G.*
object Use { val x = 1 == 2 }
