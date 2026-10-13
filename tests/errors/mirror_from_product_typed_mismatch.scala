// scala-library's `fromProductTyped` takes a product whose mirror's element types are within the
// receiver's (`Elems <: p.MirroredElemTypes`), which the synthesized mirror's refinement is
// checked against (`Synthesizer.checkRefinement`).
// expect: 8:92: error: MirroredElemTypes mismatch, expected: (String), found:  = Elems.
// expect: 1 error found
import scala.deriving.Mirror
case class P(x: Int)
@main def run(): Unit = println(summon[Mirror.ProductOf[P]].fromProductTyped(Tuple1("bad")))
