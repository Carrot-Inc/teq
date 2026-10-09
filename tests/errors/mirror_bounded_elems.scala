// expect: 11:37: error: no given instance of type ProductOf[P]{type MirroredElemTypes >: (Any, String)} was found for parameter m
// expect: 1 error found
// A mirror whose element types miss the bounds of the refinement is no given; the bounds
// `Mirror` declares for its members are met by every mirror.
import scala.deriving.Mirror
case class P(i: Int, s: String)

def make[A <: Tuple, B](v: A)(using m: Mirror.ProductOf[B] { type MirroredElemTypes >: A }): B = m.fromTuple(v)

@main def main(): Unit =
  make[(Any, String), P](("a", "b"))
  println(make[(Int, String), P]((1, "b")))
