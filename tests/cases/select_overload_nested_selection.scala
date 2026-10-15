// A selection on the selected member's result (`O.f.m.length`): deepened, the `m` of `C` gives an `Int` without
// `length`, `D`'s a `String` with it.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def m: Int = 1 }
class D { def m: String = "abcdefg" }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f.m.length)
