import scala.language.implicitConversions
object P {
 implicit val n: Int = 1
 implicit def conv(x: Int): String = x.toString
 implicit class Wrap(val i: Int) { def twice = i * 2 }
 def f(implicit n: Int): Int = n
 implicit def make[A](implicit a: A): Option[A] = Some(a)
 implicit lazy val l: Long = 2L
 implicit object O
 class C(implicit val x: Int)
}
