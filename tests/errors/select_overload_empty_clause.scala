// A method with an empty clause normalizes to its result where `() => C` lacks the member (`ProtoTypes.normalize`'s
// `ft frozen_<:< pt`): `f()`'s `C` has `++`, so `f()` is selected, and its reference without `()` is refused as
// scalac refuses it.
// expect: 14:33: error: method f in object O must be called with () argument
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
class D { def other(n: Int): Int = n }
object O:
  def f(): C = new C
  def f(using T): D = new D
@main def run(): Unit = println(O.f ++ 7)
