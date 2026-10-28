import scala.annotation.nowarn
object N0 {
  @nowarn("msg=interpolation uses toString")
  def text(x: Object): String = s"$x"
}
