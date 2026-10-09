// expect: 9:22: error: no given instance of type ProductOf[P]{type MirroredMonoType = String} was found for parameter m
// expect: 1 error found
// A mirror target whose mono type is not the mirrored class is no given.
import scala.deriving.Mirror
case class P(i: Int)

def bad(using m: Mirror.ProductOf[P] { type MirroredMonoType = String }): String = m.fromProduct(Tuple1(1))
@main def main(): Unit =
  val s: String = bad
  println(s)
