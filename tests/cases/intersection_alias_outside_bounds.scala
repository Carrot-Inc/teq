// An intersection's member that is an alias on one side and bounds the alias does not meet on the
// other has the two intersected (`>: Int | String <: Int`), on either side, as scalac's `&` of a
// `TypeAlias` and `TypeBounds`: both a `String` and an `Int` are values of it.
trait P { type T >: String }
trait Q { type T = Int }

object Members:
  def a(q: P & Q): q.T = "s"
  def b(q: P & Q): q.T = 1
  def c(r: Q & P): r.T = "t"
  def d(r: Q & P): r.T = 2

@main def Main(): Unit = println(Members.toString.nonEmpty)
