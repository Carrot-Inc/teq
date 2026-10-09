// `DummyImplicit` tells apart two overloads that erase alike, and is found without an import.
object Main:
  def f(xs: List[Int]): String = "ints " + xs.sum
  def f(xs: List[String])(implicit d: DummyImplicit): String = "strings " + xs.mkString
  def g(using DummyImplicit): Int = 1
  def main(args: Array[String]): Unit =
    println(f(List(1, 2)))
    println(f(List("a", "b")))
    println(g)
    println(summon[DummyImplicit] != null)
