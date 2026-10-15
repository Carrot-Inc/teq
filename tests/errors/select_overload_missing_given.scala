// No given `T` for the implicit-only alternative, which the selection of `&&` leaves alone (the defaulted
// `f(n: Int = 1)` normalizes to `Int => C`, which has no `&&`): the missing given is reported, as scalac
// reports it, rather than the defaulted alternative selected.
// expect: 10:36: error: no given instance of type T was found for parameter x$1
trait T
class C { def &&(n: Int): Int = n }
object O:
  def f(using T): C = new C
  def f(n: Int = 1): C = new C
@main def run(): Unit = println(O.f && 6)
