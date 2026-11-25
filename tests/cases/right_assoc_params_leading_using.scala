// A right-associative extension's leading using clause stays before the swapped clauses: written out, the using
// argument comes first, then the right operand, then the receiver.
case class U(t: String)
extension (using u: U)(x: Int)
  def +:(y: String): String = y + x + u.t
@main def run(): Unit =
  println(+:(using U("w"))("s")(1))
  given U = U("g")
  println(1 +: "i")
