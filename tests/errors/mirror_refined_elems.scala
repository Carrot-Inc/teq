// expect: 10:78: error: MirroredElemTypes mismatch, expected: (String, Int), found:  = (Int, Int).
// expect: 1 error found
// A mirror refined with element types the class does not have is reported as scalac's
// `Synthesizer.checkRefinement` reports it; the refinement naming `MirroredType` is under the
// `ProductOf` alias.
import scala.deriving.Mirror
case class P(name: String, age: Int)

@main def main(): Unit =
  val m = summon[Mirror.ProductOf[P] { type MirroredElemTypes = (Int, Int) }]
  println(m)
