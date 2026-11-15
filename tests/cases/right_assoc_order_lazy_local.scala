// A local lazy value on the LEFT of `::` is read (its initializer run) before the right operand's effect: it is no
// pure expression (`TreeInfo.exprPurity`: an idempotent path) and `typedInfixOp` binds it. Master left it in place.
object Log:
  private var s: String = ""
  def apply(x: String): Unit = { s = s + x }
  def take(): String = { val r = s; s = ""; r }
class Elem(val n: Int) { override def toString: String = "E" + n }
def line(tag: String, r: Any): Unit = println(tag + " " + Log.take() + " " + r)
def two(a: Any, b: Any): String = a.toString + b.toString
def rd(): Int = { Log("r"); 0 }
@main def main(): Unit =
  lazy val lz: Elem = { Log("l"); new Elem(1) }
  line("n5", lz :: { Log("r"); List[Any](0) })
