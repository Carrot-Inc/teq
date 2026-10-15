// No view of either alternative's result gives a `++` that takes the `Int`: none matches, as scalac reports.
// expect: 17:33: error: None of the overloaded alternatives of method f in object O
import scala.language.implicitConversions
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C
class D
class CC { def ++(s: String): Int = s.length }
class DD { def ++(s: String): Int = s.length }
given Conversion[C, CC] = _ => new CC
given Conversion[D, DD] = _ => new DD
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f ++ 7)
