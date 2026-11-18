import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) =
    val f = (x: A) => Seq(x, b)
    f(a).head

