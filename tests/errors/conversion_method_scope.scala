// expect: 11:29: error: ambiguous given instances for Int => String: a, b
// Conversion methods are implicit values of the function type at their own scope: two of the inner
// object are ambiguous before an outer implicit function is considered (scalac: E172 Ambiguous given
// instances: both method a in object Inner and method b in object Inner match type Int => String).
import scala.language.implicitConversions
implicit val outer: Int => String = i => s"outer $i"
def use(i: Int)(implicit f: Int => String): String = f(i)
object Inner:
  implicit def a(i: Int): String = s"a $i"
  implicit def b(i: Int): String = s"b $i"
  def run(): String = use(1)
@main def main(): Unit = println(Inner.run())
