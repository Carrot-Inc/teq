// scala-library's `fromProductTyped` (`Mirror.scala`): an extension of a product's mirror whose
// `Elems <: p.MirroredElemTypes`, a bound naming the receiver, the using clause's refinement
// fixes from the argument's own mirror (`Synthesizer.makeProductMirror`'s `checkRefinement`).
import scala.deriving.Mirror
case class P(x: Int)
case class Q(a: Int, b: String)
case class R(n: Int, s: String)
@main def run(): Unit =
  val p = summon[Mirror.ProductOf[P]]
  println(p.fromProductTyped(Tuple1(1)))
  val q = summon[Mirror.ProductOf[Q]]
  println(q.fromProductTyped((2, "two")))
  println(q.fromProductTyped(R(3, "three")))
  println(Mirror.fromProductTyped(q)(R(4, "four")))
  println(q.fromTuple((5, "five")))
