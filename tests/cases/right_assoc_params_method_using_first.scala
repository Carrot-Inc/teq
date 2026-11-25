// A right-associative extension whose method's first clause is a using clause is not swapped: written out, the
// receiver comes first.
case class U(t: String)
extension (x: Int)
  def +:(using u: U)(y: String): String = y + x + u.t
@main def run(): Unit = println(+:(1)(using U("w"))("s"))
