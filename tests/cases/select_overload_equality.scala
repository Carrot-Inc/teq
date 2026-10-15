// `==` on an overloaded reference: `Any`'s member, on every alternative.
trait T
object E { given T = new T {} }
import E.given
class C
object O:
  def f(using T): C = new C
  def f(n: Int): C = new C
@main def run(): Unit = println(O.f == null)
