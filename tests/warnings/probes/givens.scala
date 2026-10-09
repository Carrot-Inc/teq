// The implicit search, a conversion and an extension method use their imports.
import scala.language.implicitConversions
object O { given Int = 1; given s: String = "s" }
object Conv { given Conversion[Int, String] = _.toString }
object P { extension (s: String) def twice = s + s; extension (s: String) def thrice = s * 3 }
object UseGiven {
  import O.given
  def f = summon[Int]
}
object UseConv {
  import Conv.given
  def g: String = 1
}
object UnusedConv {
  import Conv.given
  def g: String = "1"
}
object UseExt {
  import P.twice
  import P.thrice
  def h = "x".twice
}
object NamedGiven {
  import O.s
  def k = summon[String]
}
