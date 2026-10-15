// A deeper selection's operand typed as an overloaded application's argument (pretype_value): a written literal
// by its literal type, which a parameter of a literal type takes.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def g(k: "k"): String = "c" }
class D { def g(n: "v"): String = "d" }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit =
  println(O.f.g("k"))
  println(O.f.g("v"))
