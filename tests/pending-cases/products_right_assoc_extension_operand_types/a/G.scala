package ga
object G:
  extension [T](r: T) def +::(a: List[T]): List[T] = r :: a
  extension (r: Int) def +:::(a: List[Int]): List[Int] = r :: a
  extension [T](r: T) def +++(a: List[T]): List[T] = r :: a
