// The marker is `scala.compiletime.deferred` by its symbol (dotty's `Namer`, 1929-1937): a
// program's own `deferred` makes an ordinary given of the trait, whose value is its call.
object Local:
  def deferred: Int = 13
import Local.deferred

trait T:
  given x: Int = deferred
class C extends T

@main def run(): Unit = println((new C).x)
