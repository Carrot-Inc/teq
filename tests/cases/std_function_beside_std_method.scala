// A class that is a function of two arguments and a `java.util.function.Function` answers each `apply` by its own,
// directly, through either type and as a function value, and a plain function value beside it is called as before:
// the line scalac 3.8.4 prints.
class Both extends ((Int, Int) => Int) with java.util.function.Function[String, String] {
  def apply(x: Int, y: Int): Int = x + y
  def apply(s: String): String = "name:" + s
}
object Main {
  def main(args: Array[String]): Unit = {
    val b = new Both
    val f: (Int, Int) => Int = _ * _
    println(List[Any](b.apply(1, 2), b.apply("x"), (b: (Int, Int) => Int)(3, 4), (b: java.util.function.Function[String, String]).apply("y"), f(3, 4), List(1, 2).reduce(b)).mkString(" "))
  }
}
