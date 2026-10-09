// A mirror asked for through `Mirror.ProductOf[P]` with the element types refined, as skunk's
// `Iso.productInstance` asks for it: the refinement naming `MirroredType` is under the alias.
import scala.deriving.Mirror

trait Iso[A, B]:
  def to(a: A): B
object Iso:
  def instance[A, B](t: A => B): Iso[A, B] = new Iso[A, B] { def to(a: A) = t(a) }
  given productInstance[A <: Tuple, B <: Product](using m: Mirror.ProductOf[B] { type MirroredElemTypes = A }): Iso[A, B] =
    instance[A, B](m.fromProduct)

case class P(name: String, age: Int)

def conv[D](using ev: Iso[(String, Int), D]): D = ev.to(("x", 1))

@main def main(): Unit =
  println(conv[P])
  val m = summon[Mirror.ProductOf[P] { type MirroredElemTypes = (String, Int) }]
  println(m.fromProduct(("y", 2)))
