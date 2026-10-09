// `map` over a string gives a string for a `Char` result and an `IndexedSeq` for any other, as
// scala-library's StringOps has both.
@main def run(): Unit =
  val codes = "ab".map(_.toInt)
  println(codes)
  println(codes.sum)
  val upper: String = "ab".map(_.toUpper)
  println(upper)
  println("xyz".map(c => c.toString * 2))
  println("hi".map(c => (c + 1).toChar))
