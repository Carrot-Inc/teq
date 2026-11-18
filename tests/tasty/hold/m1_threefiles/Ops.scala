class Ops[A](a: A):
  def m(b: A) = Twice.twice(Seq(a, b)).head.head
object Twice:
  inline def twice[T](x: T): List[T] = List(x, x)
