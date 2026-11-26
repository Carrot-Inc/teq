// A right-associative extension's own context bound counts among its type arguments.
trait TC[A]:
  type T
object Lib:
  extension [A](x: A)
    def %:[B: TC as ev](y: ev.T): String = y.toString + x.toString
given evidence: TC[Int] with
  type T = Int
@main def run(): Unit = println(Lib.%:[String, Int](using evidence)(7)("r"))
// expect: 9:39: error: Too many type arguments for Lib.%:[A]
