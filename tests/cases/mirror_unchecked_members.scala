import scala.deriving.Mirror

// scalac checks a mirror against its target's element types, element labels and mono type
// alias only: a label, a bound on the mono type or on the mirrored type pass unchecked.
case class P(i: Int)

@main def main(): Unit =
  val a = summon[Mirror.ProductOf[P] { type MirroredLabel <: Tuple }]
  val b = summon[Mirror.ProductOf[P] { type MirroredMonoType <: String }]
  val c = summon[Mirror.ProductOf[P] { type MirroredLabel = "Q" }]
  val d = summon[Mirror.ProductOf[P] { type MirroredType <: String }]
  val e = summon[Mirror.ProductOf[P] { type MirroredElemLabels = "i" *: EmptyTuple }]
  println(List(a, b, c, d, e).map(_.fromProduct(Tuple1(7))))
