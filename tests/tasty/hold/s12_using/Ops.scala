import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A)(using o: Ordering[A]) = List(a, b).sorted(using o).head
  given gl(using o: Ordering[A]): List[A] = List(a)

