// target_name_unrelated_value with `@targetName` written: master dropped `Other.value()` here too
// (a call of `value` looked for the source name `f`), on JavaScript and on the JVM.
import scala.annotation.targetName
class Other:
  def value(): String = "other"
class Methods:
  @targetName("value") def f(x: Int): String = "int"
  @targetName("text") def f(x: String): String = "string"
@main def test(): Unit =
  val m = new Methods
  println(m.f(1))
  println(m.f("s"))
  println(new Other().value())
