trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def foo(n: Int): Int = n }
class D { def foo(n: String): Int = n.length }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f.foo(n = M.arg))
