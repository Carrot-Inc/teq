// expect: 8:75: error: no given instance of type ProductOf[P]{type MirroredElemTypes <: String} was found for parameter x
// expect: 1 error found
// A bound on the element types other than `Mirror`'s own `<: Tuple` is checked.
import scala.deriving.Mirror
case class P(i: Int)

@main def main(): Unit =
  val m = summon[Mirror.ProductOf[P] { type MirroredElemTypes <: String }]
  val l = summon[Mirror.ProductOf[P] { type MirroredLabel <: String; type MirroredElemLabels <: Tuple }]
  println(l.fromProduct(Tuple1(1)))
