// Both alternatives' results have `&&`: the shallow selection leaves both, and the deeper prototype with the
// argument picks the one whose `&&` takes an `Int` (`resolveOverloaded1`'s `deepenProto`).
trait T
trait U
object E { given T = new T {}; given U = new U {} }
class C { def &&(n: Int): Int = n }
class D { def &&(s: String): Int = s.length }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
import E.given
@main def run(): Unit = println(O.f && 6)
