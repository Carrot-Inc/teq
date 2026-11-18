import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) = List(List(a), List(b)).flatten.head

