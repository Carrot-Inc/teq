object Lib {
  @deprecated("old alias", "1") type Old[A] = List[A]
}
object Main {
  def f(xs: Lib.Old[Int]): Int = xs.size
  def main(args: Array[String]): Unit = println(f(List(1)))
}
