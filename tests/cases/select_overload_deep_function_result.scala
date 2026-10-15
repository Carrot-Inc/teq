// A function's apply is tested against the whole member prototype, the expected result included
// (SelectionProto.isMatchedBy): (Int => Int).apply(1) gives an Int, not the String expected; C's apply gives one.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def apply(n: Int): String = "ok" }
object O:
  def f(using T): C = new C
  def f(using U): Int => Int = x => x
@main def run(): Unit =
  val s: String = O.f.apply(1)
  println(s)
