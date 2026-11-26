// A bound of a right-associative extension's own type parameter cannot read the receiver, which its clauses follow.
trait Box:
  type T
object Lib:
  extension (x: Box)
    def %:[A <: x.T](y: A): A = y
@main def run(): Unit = println("ok")
// expect: 6:17: error: Not found: x
