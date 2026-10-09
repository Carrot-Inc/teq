package erased

object B:
  def number(xs: List[Int]): Int = A.first(xs)
  def word(xs: List[String]): String = A.first(xs)
