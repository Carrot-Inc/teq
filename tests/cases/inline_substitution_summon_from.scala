// `summonFrom` reduced over its stored cases, as scalac's `InlineReducer` reduces it: each case's
// pattern type searched at the call site in source order, a failed search going on to the next
// case, a found given bound once to the case's binder, which is a given in the case's body
// (scalac reads `x: T` as `given x @ _: T`; the retype path does not);
// an inline given holding a `summonFrom`, found by a search a body's `summonInline` makes, sees
// the body's import (the kittens shape). scalac prints the lines of the .expected file.
import scala.compiletime.{summonFrom, summonInline}
final class Show[A](val show: A => String)
object Show:
  inline def apply[A]: Show[A] =
    import Instances.given
    summonInline[Show[A]]
object Instances:
  given Show[Int] = new Show(a => s"int $a")
  inline given pair[A, B]: Show[(A, B)] = summonFrom {
    case sa: Show[A] =>
      summonFrom {
        case sb: Show[B] => new Show(p => sa.show(p._1) + ", " + sb.show(p._2))
        case _ => new Show(p => sa.show(p._1) + ", no show")
      }
    case _ => new Show(_ => "no show for the first")
  }
trait Marker:
  def name: String
object HasMarker:
  given Marker with
    def name = "site's"
def viaSummon(using m: Marker): String = "summoned " + m.name
inline def pick: String = summonFrom {
  case x: Marker => viaSummon
  case _: String => "a string"
  case _ => "none"
}
inline def describe[T](t: T): String = summonFrom {
  case s: Show[T] => "shown " + s.show(t)
  case _ => "no show for " + t
}
inline def twice: String =
  val first = summonFrom { case m: Marker => m.name; case _ => "none" }
  first + " then " + summonInline[Marker].name
@main def run(): Unit =
  println(Show[(Int, Int)].show((1, 2)))
  println(Show[(Int, String)].show((3, "s")))
  println(pick)
  import Instances.given
  println(describe(4))
  println(describe("s"))
  import HasMarker.given
  println(pick)
  println(twice)
