// `scala.math.round`'s overloads (scala-library's
// `package.scala`): `round(x: Float): Int`,
// `round(x: Double): Long` and the deprecated
// `round(x: Int): Int` and `round(x: Long): Long`.
// Waiting on the std's declarations, a part of their own.
@main def run(): Unit =
  val f: Int = math.round(1.5f)
  val l: Long = math.round(1234567890123456789L): @annotation.nowarn
  println(s"$f $l")
