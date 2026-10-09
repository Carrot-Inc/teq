// jars: fixtures
// Expression quote patterns of a macro jar compiled by Scala 3.3 (TASTy 28.3), where they are
// pickled as an `unapply` of `QuoteMatching.ExprMatch`: a hole ascribed a type with pattern
// variables, one in an intersection with the macro's type parameter, a nested match naming the
// outer case's variable, and a pattern without holes.
import fix.qpat.{LayerBox, LayerMacros}

object Main:
  val settings: LayerBox[Any, Nothing, Long] = new LayerBox("settings")
  val greeter: LayerBox[Long, Throwable, Int] = new LayerBox("greeter")
  val other: LayerBox[Boolean, Nothing, Char] = new LayerBox("other")

  def main(args: Array[String]): Unit =
    println(LayerMacros.describe(settings))
    println(LayerMacros.describe(greeter))
    println(LayerMacros.chain(settings, greeter))
    println(LayerMacros.chain(settings, other))
    println(LayerMacros.isUnit(()))
    println(LayerMacros.isUnit(1))
