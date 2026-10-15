// `sort` is one of the JavaScript array's members std/core.scala gives `Array` beside the ones Array.scala
// declares: Scala's `Array` has none, so only `C` matches the selection; `length` is `Array`'s own.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def sort(): Int = 7 }
object O:
  def f(using T): C = new C
  def f(using U): Array[Int] = Array(1, 2)
@main def run(): Unit =
  println(O.f.sort())
  println(O.f.length)
