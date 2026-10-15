// An argument whose typing fails leaves the alternatives as they were: the ambiguity is reported, as scalac does.
// expect: 13:33: error: Ambiguous overload. The overloaded alternatives of method f in object O
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
class D { def ++(s: String): Int = s.length }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
transparent inline def arg: Int = scala.compiletime.error("argument failed")
@main def run(): Unit = println(O.f ++ arg)
