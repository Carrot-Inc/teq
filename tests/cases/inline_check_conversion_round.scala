// A transparent inline conversion first needed in the round of an overload's resolution that accepts conversions is
// checked as it stands in its object, outside that round: an overloaded call in its body takes the alternative that
// fits without a conversion (`kind(Any)` for an `Int`), as scalac types the body in the context it was made in, not
// the one that fits only through the `Conversion[Int, String]` in scope.
import scala.language.implicitConversions

class Plain
class Wide
class Other

object Rounds:
  var seen: String = ""
  def kind(x: String): String = "string"
  def kind(x: Any): String = "any"
  given Conversion[Int, String] = _.toString
  def taken(w: Wide): String = "wide " + seen
  def taken(o: Other): String = "other " + seen
  def run: String = taken(new Plain)
  transparent inline implicit def widened(p: Plain): Wide = { seen = kind(1); new Wide }

@main def run(): Unit = println(Rounds.run)
