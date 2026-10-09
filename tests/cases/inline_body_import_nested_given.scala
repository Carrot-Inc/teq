// An inline given a search at an inline body's call site expands stands in the body's scopes:
// `pair`'s `summonInline[Show[A]]` finds `Instances.given`, which `apply`'s block imports. scalac
// prints `int 1, int 2`.
import scala.compiletime.summonInline
trait Show[A]:
  def show(a: A): String
object Show:
  inline def apply[A]: Show[A] =
    import Instances.given
    summonInline[Show[A]]
object Instances:
  given Show[Int] with
    def show(a: Int) = s"int $a"
  inline given pair[A, B]: Show[(A, B)] = new Show[(A, B)]:
    def show(p: (A, B)) = summonInline[Show[A]].show(p._1) + ", " + summonInline[Show[B]].show(p._2)
@main def run(): Unit = println(Show[(Int, Int)].show((1, 2)))
