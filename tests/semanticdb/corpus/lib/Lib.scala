package probe
trait Base:
  def f(x: Boolean): String = "boolean"
class Lib extends Base:
  def f(x: String): String = "string"
  def f(x: Int): String = "int"
  def f(x: Long): String = "long"
  def f(x: Double): String = "double"
  def m[A](x: A): A = x
  def m(x: String): String = x
  val opt: Option[Int] = None
object Lib:
  def f(x: String): String = x
  def f(x: Int): Int = x
case class CC(x: Int)
object CC:
  def apply(s: String): CC = new CC(s.length)
