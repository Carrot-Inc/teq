// An object with an initializer as an operand of a right-associative extension called infix: its first reference
// runs the initializer, so scalac lifts it as the left operand (`refPurity`: an object with initialisers is no pure
// path), and it loads before the right operand; as the right operand it loads after the left one. A value class's
// instance extension and a top-level one, each object referenced first in its own line.
object Log { var out = List.empty[String]; def apply(s: String): Unit = out = out :+ s }
object Mod { Log("l"); override def toString = "Mod" }
object Top { Log("t"); override def toString = "Top" }
object Kept { Log("k"); override def toString = "Kept" }
object Plain { override def toString = "Plain" }
class V(val n: Int) extends AnyVal {
  extension (x: Any) def +::(y: String): String = y + x + n
  extension (x: Int) def -::(k: Kept.type): String = "" + x + k + n
}
extension (x: Any) def *::(y: String): String = y + x
object Main {
  def rd(): String = { Log("r"); "r" }
  def num(): Int = { Log("n"); 1 }
  def show(label: String, r: Any): Unit = { println(label + " " + r + " " + Log.out.mkString(",")); Log.out = Nil }
  def main(args: Array[String]): Unit = {
    val v = new V(7)
    import v.*
    show("value class, object left", Mod +:: rd())
    show("value class, object right", num() -:: Kept)
    show("value class, plain object left", Plain +:: rd())
    show("top level, object left", Top *:: rd())
  }
}
