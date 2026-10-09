import scala.deriving.Mirror

trait Box:
  type Elem
  def label: String

def narrow[t <: Product](x: Box { type Elem = t } & Box { type Elem <: Tuple }): Box { type Elem <: Product } = x

def widths[B, t <: Product](m: Mirror.ProductOf[B] { type MirroredElemTypes = t } & Mirror.Of[B]): Mirror.ProductOf[B] { type MirroredElemTypes <: Product } = m

case class Pair(a: Int, b: String)

@main def main(): Unit =
  val b = new Box { type Elem = (Int, Int); def label = "pair" }
  println(narrow[(Int, Int)](b).label)
  println(widths[Pair, (Int, String)](summon[Mirror.ProductOf[Pair]]).fromProduct((1, "x")))
