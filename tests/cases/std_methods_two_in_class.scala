// A class that inherits two methods of one name which two unrelated types of the standard library declare answers
// each call by the method of its type, directly and through either type: the line scalac 3.8.4 prints.
trait Fetch extends java.util.function.Supplier[String] { def get(): String = "supplied" }
trait Lookup extends java.util.function.IntFunction[String] { def apply(i: Int): String = "item" + i }
trait ByName extends java.util.function.Function[String, String] { def apply(s: String): String = "name:" + s }
class Both extends Lookup with ByName
object Main {
  def main(args: Array[String]): Unit = {
    val b = new Both
    println(List[Any](b.apply(1), b.apply("x"), (b: java.util.function.IntFunction[String]).apply(2), (b: java.util.function.Function[String, String]).apply("y")).mkString(" "))
  }
}
