// `scala.math.round`'s overloads (scala-library's `package.scala`): `round(x: Float): Int`, `round(x: Double): Long`
// and the deprecated `round(x: Int): Int` and `round(x: Long): Long`. The lean std declares the `Double` one alone
// (`std/math.scala`), so teq gives `math.round(1.5f)` a `Long` and converts a `Long` argument to a `Double` (E167
// where it loses digits, `tests/scala3-typing` `warn/t3235-minimal.scala`): the std's declarations, a part of their
// own (met by the review of `cf6cad6c1`'s item 5).
@main def run(): Unit =
  val f: Int = math.round(1.5f)
  val l: Long = math.round(1234567890123456789L): @annotation.nowarn
  println(s"$f $l")
