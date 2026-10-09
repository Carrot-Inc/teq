// An extension method whose later parameter names the receiver's path (scala-library's
// `extension (p: Mirror.Product) def fromTuple(t: p.MirroredElemTypes)`, which pureconfig's
// derivation calls): the receiver's path stands for the parameter's.
import scala.deriving.Mirror
trait M:
  type E
  def make(e: E): String
extension (p: M) def viaExt(t: p.E): String = p.make(t)
object I extends M:
  type E = Int
  def make(e: Int) = "I" + e
final case class P(a: Int, b: String)
def build[A](t: Tuple)(using m: Mirror.ProductOf[A]): A = m.fromTuple(t.asInstanceOf[m.MirroredElemTypes])
@main def main(): Unit =
  println(I.viaExt(3))
  println(build[P]((1, "a")))
