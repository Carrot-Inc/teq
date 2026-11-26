// A right-associative call whose right operand reaches the operator through a conversion, the left operand a lambda typed against the receiver's formal.
import scala.language.implicitConversions
class R:
  def %:(f: Int => Int): Int = f(41)
class Q
given Conversion[Q, R] with
  def apply(q: Q): R = { println("convert"); new R }
@main def run(): Unit = println((x => x + 1) %: new Q)
