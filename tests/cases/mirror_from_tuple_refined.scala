// `fromTuple` is scala-library's extension of `ProductOf[T]` alone (`Mirror.scala`): a receiver
// refined twice on `MirroredElemTypes` (`<: Tuple` by `ProductOf`, `>: A` by the call) is one, its
// member's bounds meeting (`RefinedType`'s member: the parent's `&` the refinement).
import scala.deriving.Mirror
case class P(i: Int, s: String)

def make[A <: Tuple, B](v: A)(using m: Mirror.ProductOf[B] { type MirroredElemTypes >: A }): B = m.fromTuple(v)

@main def run(): Unit =
  println(make[(Int, String), P]((1, "b")))
  val m = summon[Mirror.ProductOf[P]]
  println(m.fromTuple((2, "c")))
  println(Mirror.fromTuple(m)((3, "d")))
