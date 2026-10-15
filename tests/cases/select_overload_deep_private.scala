// `D`'s `foo(Int)` is private: the deeper selection tests the accessible alternatives alone (`isAccessibleFrom`),
// leaving `C`'s.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def foo(n: Int): Int = n }
class D { def foo(s: String): Int = s.length; private def foo(n: Int): Int = n }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f.foo(7))
