// A companion that writes `MirroredMonoType` concretely keeps it (`SyntheticMembers`' `monoType`): its own
// `fromProduct` and the summoned mirror's both answer by it.
import scala.deriving.Mirror
case class C(x: Int)
object C:
  type MirroredMonoType = C
  def fromProduct(p: Product): C = new C(p.productElement(0).asInstanceOf[Int] + 1)
@main def run(): Unit =
  println(C.fromProduct(Tuple1(1)))
  val m = summon[Mirror.ProductOf[C]]
  println(m.fromProduct(Tuple1(5)))
