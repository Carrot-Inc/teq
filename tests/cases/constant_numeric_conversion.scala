// A numeric conversion of a constant tree has the converted constant's type (dotty's
// `ConstFold.Select`, `foldUnop`): `2.5f.toInt` is a `2`, before adaptation and overload
// applicability, so `pick(1.5f.toInt)` chooses the overload over `1`.
import scala.annotation.targetName
@targetName("one") def pick(x: 1): String = "one"
def pick(x: Int): String = "int"
@main def run() =
  println(pick(1.5f.toInt))
  val a: 2 = 2.5f.toInt
  val b: 2L = 2.5f.toLong
  val c: 2.5 = 2.5f.toDouble
  val d: 'A' = 65.toChar
  println(a + b)
  println(c > 2)
  println(d)
