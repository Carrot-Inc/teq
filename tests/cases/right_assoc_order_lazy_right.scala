// A top-level lazy value on the RIGHT of `::` is no pure path: the impure left operand is bound ahead of it. Master
// called the lazy value stable and read it first.
object Log:
  private var s: String = ""
  def apply(x: String): Unit = { s = s + x }
  def take(): String = { val r = s; s = ""; r }
class Elem(val n: Int) { override def toString: String = "E" + n }
def line(tag: String, r: Any): Unit = println(tag + " " + Log.take() + " " + r)
def two(a: Any, b: Any): String = a.toString + b.toString
def rd(): Int = { Log("r"); 0 }
lazy val tr: List[Any] = { Log("r"); List[Any](0) }
@main def main(): Unit =
  line("n8", { Log("l"); new Elem(1) } :: tr)
