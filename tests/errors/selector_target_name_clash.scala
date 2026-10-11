// expect: selector_target_name_clash.scala:10:12: error: Name clash between defined and inherited member:
// A case class's selector `_1` against an inherited alternative whose `@targetName` is `_1` and
// whose erasure is the selector's: scalac 3.8.4 reports E120 at the class.
import scala.annotation.targetName

trait T:
  @targetName("_1") def first: Int = 0
  def first(x: Int): Int = x

case class C(a: Int) extends T
