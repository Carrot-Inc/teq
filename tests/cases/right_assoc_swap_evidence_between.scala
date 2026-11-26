// A right-associative extension whose receiver's type reads a context bound's evidence: the evidence stands before the receiver's clause.
trait TC[A]:
  type T
object Lib:
  extension [A: TC as ev](x: ev.T)
    def %:(y: String): String = y + x.toString
given evidence: TC[Int] with
  type T = Int
@main def run(): Unit = println(Lib.%:[Int]("r")(using evidence)(7))
