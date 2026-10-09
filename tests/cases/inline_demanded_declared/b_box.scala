package demandeddeclared

final class Box[A](val value: A):
  def both: List[A] = List(value, value)
  def same(a: A): A = a
  inline def twice: Int = both.size + same(value).toString.length
