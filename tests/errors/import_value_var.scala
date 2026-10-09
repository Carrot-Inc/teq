// expect: 12:10: error: (d : Dsl) is not a valid import prefix, since it is not an immutable path
// An import's qualifier is a stable path, so a `var` is none (scalac: E083).
import scala.language.implicitConversions
class Dsl(n: Int):
  def root: String = s"root $n"
  def size(s: String): String = s"len ${s.length}"
  implicit def conv(i: Int): String = "x" * i
object Build:
  def use[A](n: Int)(f: Dsl => A): A = f(new Dsl(n))
@main def main(): Unit =
  var d = new Dsl(1)
  import d.*
