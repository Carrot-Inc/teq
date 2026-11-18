// `==` and `!=` written as a selection are `Any`'s members, as the operators are (`x.==(y)` is `x == y`), on a class,
// a primitive, a string, a class deriving from `scala.Dynamic` (the name is no dynamic call), and beside a class's own
// `==` of two parameters, which takes the two arguments while `Any`'s takes one.
import scala.language.dynamics
class K
class P { def ==(x: String, y: String): String = "two" }
class Typed extends Dynamic:
  def selectDynamic(name: String): String = name
  def applyDynamic(name: String)(args: Any*): String = name
@main def run(): Unit =
  val k = new K
  println(k.==(k))
  println(k.!=(new K))
  println(true.==(false))
  println(1.!=(2))
  println("a".==("a"))
  val p = new P
  println(p.==(p))
  println(p.==("a", "b"))
  val t = new Typed
  println(t.==(t))
  println(t.!=(t))
