// expect: 9:3: error: type mismatch: found Int, required String
// A wildcard import from a value leaves its givens out, a `given Conversion` too, as it does an
// object's (scalac: E007 Found: (1 : Int), Required: String).
import scala.language.implicitConversions
class D:
  given Conversion[Int, String] = _.toString
def f(d: D): String =
  import d.*
  1
@main def main(): Unit = println(f(new D))
