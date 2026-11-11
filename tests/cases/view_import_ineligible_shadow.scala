// ContextualImplicits.computeEligible (Implicits.scala:387-392): each import's eligible conversions are found
// first (filterMatching), then combined (combineEligibles, 342-361): a later import of the same name with none
// eligible for the argument shadows nothing, so the earlier import's conversion is the view.
import scala.language.implicitConversions

final class Wrapped(val value: Int)
object IntConversions:
  implicit def wrap(x: Int): Wrapped = new Wrapped(x)
object StringConversions:
  implicit def wrap(x: String): Wrapped = new Wrapped(x.length)

object Use:
  import IntConversions.wrap
  import StringConversions.wrap
  val v: Wrapped = 7

@main def run(): Unit =
  println(Use.v.value)
