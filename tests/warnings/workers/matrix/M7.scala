import scala.annotation.nowarn
import scala.collection.mutable
object M7 {
  @deprecated("old", "1") def old: Int = 7
  def lint(x: Object): String = s"$x"
  implicit def conv(x: Int): String = x.toString
  def useOld: Int = M0.old
  def post: Int = { val unusedLocal = 1; 2 }
  @nowarn("msg=interpolation uses toString") def quiet(x: Object): String = s"$x"
  @nowarn("cat=deprecation") def unusedNowarn: Int = 3
}
