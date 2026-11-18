import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) = List(a, b).map(x => (x, 1)).head._1

