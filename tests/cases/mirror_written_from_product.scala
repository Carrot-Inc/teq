// A companion's own `fromProduct` is its mirror's (`SyntheticMembers.addMirrorSupport`'s `existingDef`),
// checked against the `MirroredMonoType` alias the companion gets beside its `Mirror.Product` parent.
import scala.deriving.Mirror

case class C(x: Int)
object C:
  def fromProduct(p: Product): C = C(p.productElement(0).asInstanceOf[Int] + 1)

@main def run(): Unit =
  println(summon[Mirror.ProductOf[C]].fromProduct(Tuple1(1)))
  println(C.fromProduct(Tuple1(5)))
