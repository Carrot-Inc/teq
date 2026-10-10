object A:
  implicit def toStr(x: Int): String = x.toString
  implicit def toLen(s: String): Int = s.length

object B:
  import scala.language.implicitConversions
  implicit def toDouble(b: Boolean): Double = if b then 1.0 else 0.0

object C:
  implicit def notOne(x: Int)(using y: String): String = y
  implicit def two(x: Int, y: Int): Int = x + y
  implicit val f: Int => Long = _.toLong
  implicit class Rich(x: Int):
    def twice = x * 2

@main def run(): Unit = println(A.toLen("ab") + C.Rich(2).twice)
