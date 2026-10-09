// A method named `value` in the output by an aliased `@targetName("value")` beside a plain
// `value()` of an unrelated class: the reach keeps the class's own `value` called under that
// name, whatever source name the output's `value` maps back to elsewhere.
import scala.annotation.{targetName as tn}
class Other:
  def value(): String = "other"
class Methods:
  @tn("value") def f(x: Int): String = "int"
  @tn("text") def f(x: String): String = "string"
@main def test(): Unit =
  val m = new Methods
  println(m.f(1))
  println(m.f("s"))
  println(new Other().value())
