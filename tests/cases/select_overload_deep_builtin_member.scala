// A builtin's member the std gives as an extension (`Array.update`, `apply`) is tested in a deeper round by its
// clause past the receiver's, as `SelectionProto.isMatchedBy` tests a class's own member: `update("x", 1)` takes
// `C`'s, `apply(0)` the array's.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C:
  def update(s: String, v: Int): Int = 7
  def apply(s: String): Int = 8
object O:
  def f(using T): C = new C
  def f(using U): Array[Int] = Array(1)
@main def run(): Unit =
  println(O.f.update("x", 1))
  println(O.f.apply(0))
