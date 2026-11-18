import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) = List(a, b).sortBy(_.hashCode).head

