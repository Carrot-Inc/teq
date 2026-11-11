//> using options -experimental
// A member overriding a tracked one is tracked: it takes its right-hand side's type unwidened, not the type of the
// member it overrides (dotty's Namer.inferredResultType, `isTracked`), so `x.b.value` is an Int.
import scala.language.experimental.modularity
trait Box:
  type T
  def value: T
object IntBox extends Box:
  type T = Int
  def value = 3
class A(tracked val b: Box)
class B extends A(IntBox):
  override val b = IntBox
@main def run(): Unit =
  val x = new B
  val i: Int = x.b.value
  println(i)
