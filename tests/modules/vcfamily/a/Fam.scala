package vcf

// The members scalac synthesizes for a value class are its companion's `hashCode$extension` and
// `equals$extension`, and a case one's product members too: a scalac-built downstream calls them.
class D(val u: Double) extends AnyVal:
  def twice: Double = u * 2
case class CV(x: Int) extends AnyVal
class S(val s: String) extends AnyVal
