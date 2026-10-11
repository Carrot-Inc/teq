// Two alternatives no argument tells apart (`A[T]` and `B` both take an `AB`), resolved again
// against the expected result (`resolveOverloaded1`'s `deepPt`): the arguments fix `T` to `Int`
// first, so the generic one's `Int` does not conform to `String`, and `f(b: B)` is chosen; in a
// later argument list the earlier lists fix nothing (`resolveMapped`).
trait A[T]
trait B
class AB extends A[Int] with B

object O:
  def f[T](a: A[T]): T = ???
  def f(b: B): String = "plain"
  def g[T](x: Int)(a: A[T]): T = ???
  def g(x: Int)(b: B): String = "curried"
  val result: String = f(new AB)
  val curried: String = g(0)(new AB)

@main def main(): Unit =
  println(O.result)
  println(O.curried)
