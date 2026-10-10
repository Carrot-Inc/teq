// expect: 14:33: error: value fromTuple is not a member of M
// expect: 1 error found
// `fromTuple` is scala-library's extension of `ProductOf[T]` alone: a mirror whose `MirroredType`
// and `MirroredMonoType` differ is no `ProductOf[T]`, so it has none, as scalac finds.
import scala.deriving.Mirror

object M extends Mirror.Product:
  type MirroredType = Int
  type MirroredMonoType = String
  type MirroredLabel = "M"
  type MirroredElemTypes = Tuple1[Int]
  type MirroredElemLabels = Tuple1["x"]
  def fromProduct(p: Product): String = "wrong"
@main def run(): Unit = println(M.fromTuple(Tuple1(1)))
