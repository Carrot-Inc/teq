// expect: 12:37: error: MirroredElemTypes mismatch, expected: (Int, String), found:  >: (Any, String) <: Tuple.
// expect: 1 error found
// A mirror whose element types miss the bounds of the refinement is reported as scalac's
// `Synthesizer.checkRefinement` reports it, the member met with `Mirror`'s own `<: Tuple`; the bounds
// `Mirror` declares for its members are met by every mirror.
import scala.deriving.Mirror
case class P(i: Int, s: String)

def make[A <: Tuple, B](v: A)(using m: Mirror.ProductOf[B] { type MirroredElemTypes >: A }): B = m.fromTuple(v)

@main def main(): Unit =
  make[(Any, String), P](("a", "b"))
  println(make[(Int, String), P]((1, "b")))
