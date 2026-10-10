// A conversion method to a builtin or a value class is an implicit function value into `AnyVal`,
// which their base types leave out (scalac: "3", "7").
import scala.language.implicitConversions
class W(val n: Int) extends AnyVal
implicit def c(x: String): Int = x.length
implicit def w(x: Long): W = W(x.toInt)
@main def run(): Unit =
  val f = summon[String => AnyVal]
  val g = summon[Long => AnyVal]
  println(f("abc"))
  println(g(7L).asInstanceOf[W].n)
