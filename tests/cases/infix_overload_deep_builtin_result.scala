// A builtin operator's overloads (scala's Int.scala) are tested against the whole member prototype, the expected
// result included (SelectionProto.isMatchedBy): Int.+(Int) gives an Int, not the String expected; C's + gives one.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def +(n: Int): String = "ok" }
object O:
  def f(using T): C = new C
  def f(using U): Int = 1
@main def run(): Unit =
  val s: String = O.f + 1
  println(s)
