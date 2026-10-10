// `fromTuple` is scala-library's extension of a product's mirror over the tuple of its elements' types
// (`Mirror.fromTuple`), called on the mirror or as the method of `Mirror` with the mirror first.
import scala.deriving.Mirror
case class P(x: Int)
case class Q(a: Int, b: String)
@main def run(): Unit =
  println(Mirror.fromTuple(summon[Mirror.ProductOf[P]])(Tuple1(3)))
  val m = summon[Mirror.ProductOf[Q]]
  println(m.fromTuple((1, "one")))
