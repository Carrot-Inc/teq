// A right-associative extension whose method has a single parameter and then another clause: swapped, the
// further clause after the receiver.
extension (x: Int)
  def +:(y: String)(z: Int): String = y + x + z
@main def run(): Unit =
  println(+:("s")(1)(2))
  println((1 +: "i")(3))
