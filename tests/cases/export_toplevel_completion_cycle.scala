// A top-level wildcard export of an object whose member's inferred type creates a class of the
// file: the class's completion looks its parameter type up in the package, and a lookup of that
// name makes no forwarder of another (dotty's `computeMembersNamed` reads the members of the name
// alone), so no forwarder's inference meets the class mid-completion.
class Q(val months: Int)
object Plan:
  def zero = new Q(0)
  def one = Q(1)
export Plan.*

object user:
  case class Tier(level: Int)
  object Tier:
    def base = Tier(0)
export user.Tier.*

@main def run(): Unit =
  println(zero.months)
  println(one.months)
  println(base.level)
