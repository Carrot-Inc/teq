import scala.language.implicitConversions
class Ops[A](a: A):
  def m[B: Ordering](b: B) = List(b, b).sorted.head

