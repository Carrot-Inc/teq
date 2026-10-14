// The library's names for the std's collections (`scala.collection.Seq`, `scala.collection.immutable.List`,
// `scala.collection.Map`) in a program without a class path: the std defines them in package `scala`, and
// `STD_BINDINGS` binds the library's names to them as under a class path.
object Main:
  def f(xs: scala.collection.Seq[Int]): Int = xs.sum
  def g(xs: scala.collection.immutable.List[Int]): Int = xs.sum
  def h(xs: scala.collection.immutable.Seq[Int]): Int = xs.sum
  def k(m: scala.collection.Map[Int, Int]): Int = m.size
  def l: scala.collection.immutable.List[Int] = scala.collection.immutable.List(5, 6)

  def main(args: Array[String]): Unit =
    println(f(List(1, 2)) + g(List(3)) + h(Vector(4)) + k(Map(1 -> 2)))
    println(l)
