package ireada

// A file's eager top-level vals and var initialise together, whichever of its definitions the
// program touches first.
object Registry:
  val items: List[String] =
    println("registry")
    List("a", "b")

val top: String = "top " + Registry.items.mkString("+")
var counter: Int =
  println("counter")
  1
def twice(n: Int): Int = n * 2
