// A right-associative extension with a context bound, eta-expanded with a type argument.
trait TC[A]:
  type T
object Lib:
  extension [A: TC as ev](x: ev.T)
    def %:(y: String): String = y + x.toString
given evidence: TC[Int] with
  type T = Int
@main def run(): Unit =
  import Lib.*
  val f: Int => String = "r".%:[Int]
  println(f(7))
// expect: 11:32: error: method apply in trait Function1 does not take type parameters
