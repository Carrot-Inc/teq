// Two implicit-only alternatives whose results both declare `zap`, `A`'s private: the selection matches a member
// it can access alone (`SelectionProto.isMatchedBy`'s `isAccessibleFrom`), so `B`'s alternative is taken.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
class A { private def zap: Int = 1 }
class B { def zap: Int = 2 }
object O:
  def f(using T): A = new A
  def f(using U): B = new B
import E.given
@main def run(): Unit = println(O.f.zap)
