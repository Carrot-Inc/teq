// Parentheses keep the selection where braces drop it (dotty's `Parens` is no tree, `typedBlock` types a block's
// expression with `pt.dropIfProto`): `(O.f) ++ 7` takes `C`, whose `++` the selection reveals.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
object O:
  def f(using T): C = new C
  def f(using U): Int = 0
@main def run(): Unit = println((O.f) ++ 7)
