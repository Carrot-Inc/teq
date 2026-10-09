// `@volatile` on a field (`ACC_VOLATILE`, listed `volatile`): a class's var, one under an import's other name, a
// class parameter's, a val's (scalac warns and keeps the flag), and the var a class holds for a trait it mixes in,
// a private one's under its expanded name, as `Mixin` copies the trait member's annotations to the class's,
// and one under a type alias (`av`). A plain var's field has none, nor
// one under an annotation of the program's own named `volatile`.
// abi: x y z p q plain volatile$T$$hidden nv av
package volatile
import scala.volatile as vv
trait T:
  @volatile var x: Int = 1
  @volatile private var hidden: Int = 2
  var plain: Int = 3
  def bump(): Int = { hidden += 1; hidden }
class C(@volatile var p: Int, @volatile val q: Int) extends T:
  @volatile var y: Int = 4
  @vv var z: Int = 5
  @custom.volatile var nv: Int = 6
  @V var av: Int = 7
type V = scala.volatile
object custom:
  class volatile extends scala.annotation.StaticAnnotation
