// expect: 16:27: error: value usd is not a member of Int
// expect: 17:28: error: value usd is not a member of Long
// expect: 18:26: error: value usd is not a member of Int
// expect: 19:30: error: value usd is not a member of Char
// expect: 4 errors found
// A receiver of a numeric constant type is converted to a conversion's parameter type, as
// scalac adapts a constant; a value of a wider type, an ascribed literal among them, is not
// widened under a conversion.
import scala.language.implicitConversions
object Test:
  implicit class Ops(n: Double):
    def usd: Double = n
  val literal: Double = 44.usd + 44L.usd + -1.usd + 2.5.usd + 'a'.usd
  val n: 44 = 44
  val singleton: Double = n.usd + (-1).usd
  def f(n: Int): Double = n.usd
  def g(n: Long): Double = n.usd
  val ascribed: Double = (44: Int).usd
  val ascribedChar: Double = ('a': Char).usd
