// A right-associative extension's trailing using clause follows the receiver: written out, the right operand, the
// receiver, then the using argument.
case class U(t: String)
extension (x: Int)(using u: U)
  def +:(y: String): String = y + x + u.t
@main def run(): Unit =
  println(+:("s")(1)(using U("w")))
  given U = U("g")
  println(1 +: "i")
