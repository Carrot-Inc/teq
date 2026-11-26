// A right-associative extension with a context bound whose evidence a later clause reads: the evidence stands before its first reader.
trait TC[A]:
  type T
object Lib:
  extension [A: TC as ev](x: Int)
    def %:(y: String)(z: ev.T): String = y + x.toString + z.toString
given evidence: TC[Int] with
  type T = Int
@main def run(): Unit = println(Lib.%:[Int]("r")(7)(using evidence)(8))
