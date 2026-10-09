// An ordinary `if` whose condition is a constant selects its branch, the condition kept before
// it unless scalac's `isIdempotentExpr` holds of it: `get().b` keeps its receiver's effect, a
// lazy val's `c.b` is dropped unevaluated, and a call typed as a constant singleton path
// (`C.b.type` over a `final val b: true`) is the constant and stays. scalac prints `receiver`,
// 1, 2, `condition`, 3.
import scala.compiletime.error
object C:
  val b: true = true
  final val f: true = true
def get(): C.type = { println("receiver"); C }
def cond(): C.f.type = { println("condition"); C.f }
class L[T](x: T):
  lazy val b: T = { println("lazy"); x }
inline def receiver: Int = if get().b then 1 else error("dead")
inline def lazyCondition(c: L[true]): Int = if c.b then 2 else error("dead")
inline def singleton: Int = if cond() then 3 else error("dead")
@main def run(): Unit =
  println(receiver)
  println(lazyCondition(new L[true](true)))
  println(singleton)
