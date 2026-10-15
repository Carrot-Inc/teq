// Both results have a `foo` that is a value of a function type: the deeper selection applies its `apply` to the
// argument, `Int => Int` taking `7` and `String => Int` not.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { val foo: Int => Int = x => x }
class D { val foo: String => Int = x => x.length }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f.foo(7))
