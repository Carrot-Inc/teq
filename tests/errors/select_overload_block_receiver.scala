// A block drops the selection (dotty's `typedBlock` types its expression with `pt.dropIfProto`): `{ O.f }`, a
// single expression in braces, is typed without the selection of `++`, and its alternatives are ambiguous, as
// scalac reports, where parentheses keep the selection.
// expect: 13:36: error: Ambiguous overload. The overloaded alternatives of method f in object O
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
object O:
  def f(using T): C = new C
  def f(using U): Int = 0
@main def run(): Unit = println(({ O.f }) ++ 7)
