// An object with an initializer on the RIGHT of `::` is no pure path: the impure left operand is bound ahead of the
// object's first read, which runs the initializer. Master read the object first.
object Log:
  private var s: String = ""
  def apply(x: String): Unit = { s = s + x }
  def take(): String = { val r = s; s = ""; r }
class Elem(val n: Int) { override def toString: String = "E" + n }
def line(tag: String, r: Any): Unit = println(tag + " " + Log.take() + " " + r)
def two(a: Any, b: Any): String = a.toString + b.toString
def rd(): Int = { Log("r"); 0 }
object Mod { Log("l"); val e: Elem = new Elem(1); override def toString: String = "Mod" }
@main def main(): Unit =
  line("n12", Mod :: { Log("r"); List[Any](0) })
