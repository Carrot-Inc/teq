// The deeper selection stops at the member's first argument list: a second list does not tell the alternatives
// apart, as dotty's `FunProto` result prototype does not (scalac's ambiguity).
// expect: 13:33: error: Ambiguous overload. The overloaded alternatives of method f in object O
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def foo(n: Int)(s: String): Int = 7 }
class D { def foo(n: Int)(b: Boolean): Int = 8 }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f.foo(1)("hi"))
