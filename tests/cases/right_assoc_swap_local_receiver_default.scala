// A local right-associative extension whose receiver has a default, called with the receiver's clause left empty.
@main def run(): Unit =
  var k = 1
  extension (x: Int = 5) def +:(y: String): String = y + x + k
  println(+:("a")())
