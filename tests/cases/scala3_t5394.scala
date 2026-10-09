// Adapted from scala3 tests/run/t5394.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
  def main(args: Array[String]): Unit = ()
  def f[T](l: List[T]): Int = l match { case x :: xs => f(xs) case Nil => 0 }
  f(List.fill(10000)(0))
}
