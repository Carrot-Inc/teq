// An assignment-shaped operator is a member selection first (`O.f.+=(7)`, `Applications.typedApply` before
// `typedOpAssign`): its left operand typed under the selection of `+=`, which `C` alone has.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def +=(n: Int): Int = n }
class D
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f += 7)
