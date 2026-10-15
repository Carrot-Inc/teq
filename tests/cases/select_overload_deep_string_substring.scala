// The JDK's String.substring(int) and substring(int, int) beside C's substring(String): each call takes the
// alternative whose substring takes its arguments.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def substring(s: String): String = s }
object O:
  def f(using T): C = new C
  def f(using U): String = "abc"
@main def run(): Unit =
  println(O.f.substring(1))
  println(O.f.substring(1, 2))
  println(O.f.substring("ok"))
