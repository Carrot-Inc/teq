// The member's own expectation carried in the selection (`FunProto(args, IgnoredProto(pt))`, deepened twice): both
// `foo(Int)` take `1`, the `Int` expected picks the one whose result is an `Int`.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def foo(n: Int): Int = 7 }
class D { def foo(n: Int): String = "hi" }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit =
  val n: Int = O.f.foo(1)
  println(n)
