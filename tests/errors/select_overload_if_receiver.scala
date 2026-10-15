// An `if`'s branches are typed without the selection (dotty's `typedIf`): the overloaded reference in one is
// ambiguous, as scalac reports.
// expect: 12:47: error: Ambiguous overload. The overloaded alternatives of method f in object O
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
object O:
  def f(using T): C = new C
  def f(using U): Int = 0
@main def run(): Unit = println((if true then O.f else new C) ++ 7)
