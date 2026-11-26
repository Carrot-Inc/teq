// A right-associative extension whose receiver's default reads a context bound's evidence, called with every clause written.
trait TC[A]:
  type T
object Lib:
  extension [A: TC as ev](x: ev.T = throw new RuntimeException("default"))
    def %:(y: String): String = y + x.toString
given evidence: TC[Int] with
  type T = Int
@main def run(): Unit = println(Lib.%:[Int]("r")(using evidence)(7))
