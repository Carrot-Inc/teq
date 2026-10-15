// An implicit-only alternative whose result is a context function beside an explicit one, the left operand of `++`.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
object O:
  def f(using T): U ?=> C = new C
  def f(n: Int): Int = n
@main def run(): Unit = println(O.f ++ 7)
