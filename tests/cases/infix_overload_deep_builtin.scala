// Both results have `+`; the deeper selection tests each against the argument (`SelectionProto.isMatchedBy`'s
// compatibility): `Int`'s builtin `+` takes no `Boolean`, `C`'s does.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def +(b: Boolean): Int = 7 }
object O:
  def f(using T): Int = 1
  def f(using U): C = new C
@main def run(): Unit = println(O.f + true)
