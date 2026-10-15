// A String's trim is the JDK's trim(), not the std's implementation's trim(Unit = ()): an argument fits only C's.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def trim(u: Unit): String = "ok" }
object O:
  def f(using T): C = new C
  def f(using U): String = "abc"
@main def run(): Unit = println(O.f.trim(()))
