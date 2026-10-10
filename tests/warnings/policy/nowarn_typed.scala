// `@nowarn`'s filter is the constant string of its argument's type, typed where the annotation
// stands (`Annotation.argumentConstantString(0)`): a `final val`, a concatenation, the argument
// by name, a call whose result type is a literal type whatever it does; a value that is no
// constant is a warning, and its annotation suppresses nothing.
import scala.annotation.nowarn
object Filters {
  final val unused = "msg=unused"
  val notConstant = "msg=unused"
  def filter(i: Int): "msg=unused" = { println(i); "msg=unused" }
}
object Main {
  var n = 0
  @nowarn(Filters.unused) def a(x: Int): Int = n
  @nowarn("msg=" + "unused") def b(x: Int): Int = n
  @nowarn(value = "msg=unused") def c(x: Int): Int = n
  @nowarn(Filters.notConstant) def d(x: Int): Int = n
  def e(x: Int): Int = n
  @nowarn(Filters.filter(0)) def g(x: Int): Int = n
  def main(args: Array[String]): Unit = println(Seq(a(0), b(0), c(0), d(0), e(0), g(0)))
}
