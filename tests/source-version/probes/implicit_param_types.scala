import scala.language.implicitConversions
object O {
  implicit def f(x: => Int): String = x.toString
  implicit def g(x: Int*): String = ""
  implicit class W(x: => Int) { def twice: Int = x + x }
}
