// A companion's own `fromProduct` read against the `MirroredMonoType` it inherits concretely (`SyntheticMembers`'
// `monoType` reuses a member of the name that is no `Deferred` one): an `Int` there.
import scala.deriving.Mirror
trait Alias:
  type MirroredMonoType = Int
case class C(x: Int)
object C extends Alias:
  def fromProduct(p: Product): Int = 42
@main def run(): Unit = println(C.fromProduct(Tuple1(1)))
