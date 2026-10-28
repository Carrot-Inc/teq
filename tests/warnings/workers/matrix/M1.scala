import scala.annotation.nowarn
import scala.collection.mutable
object M1 {
  @deprecated("old", "1") def old: Int = 1
  def lint(x: Object): String = s"$x"
  implicit def conv(x: Int): String = x.toString
  def useOld: Int = M2.old
  def post: Int = { val unusedLocal = 1; 2 }
  @nowarn("msg=interpolation uses toString") def quiet(x: Object): String = s"$x"
  @nowarn("cat=deprecation") def unusedNowarn: Int = 3
}
