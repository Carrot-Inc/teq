// A boxed number's `toInt`, `toLong` or `toDouble` goes through Predef's unboxing conversion to
// the primitive, which has those conversions without a member symbol.
@main def run(): Unit =
  val i: java.lang.Integer = 5
  val d: java.lang.Double = 2.5
  println(Option(i).map(_.toInt))
  println(Option(d).map(_.toDouble))
  println(i.toLong + 1)
  println(d.toInt)
