// A function's andThen is std/core.scala's extension, tested by its clause as a member is: an Int takes C's,
// a function literal the function's, whose result is the function expected.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def andThen(n: Int): String = "ok" }
object O:
  def f(using T): C = new C
  def f(using U): Int => Int = x => x
@main def run(): Unit =
  println(O.f.andThen(1))
  val g: Int => String = O.f.andThen(_.toString)
  println(g(5))
