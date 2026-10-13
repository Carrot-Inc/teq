// A member only one part of an intersection has, through that part's refinement: a type alias
// read by a path (`r.T` is `Int`), and a structural method the refinement adds (`r.extra`, a
// reflective call), as dotty's `findMember` takes the refined part's member (`goAnd`, `goRefined`).
import scala.reflect.Selectable.reflectiveSelectable
trait A
trait B
def member(r: (A { type T = Int }) & B)(x: r.T): Int = x
def added(r: (A { def extra: Int }) & B): Int = r.extra
class AB extends A with B:
  type T = Int
  def extra: Int = 41
@main def run(): Unit =
  val ab = new AB
  println(member(ab)(7) + 1)
  println(added(ab))
