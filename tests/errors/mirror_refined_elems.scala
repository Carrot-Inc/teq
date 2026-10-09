// expect: 9:78: error: no given instance of type ProductOf[P]{type MirroredElemTypes = (Int, Int)} was found for parameter x
// expect: 1 error found
// A mirror refined with element types the class does not have is no given, as scalac rejects
// it; the refinement naming `MirroredType` is under the `ProductOf` alias.
import scala.deriving.Mirror
case class P(name: String, age: Int)

@main def main(): Unit =
  val m = summon[Mirror.ProductOf[P] { type MirroredElemTypes = (Int, Int) }]
  println(m)
