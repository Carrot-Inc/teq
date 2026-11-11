package uxa

// A user's extractors, of a tuple, of one value and of a sequence, whose matches the bodies below make.
object Pair:
  def unapply(s: String): Option[(String, String)] = Some((s.take(1), s.drop(2)))
object Words:
  def unapplySeq(s: String): Option[Seq[String]] = Some(List(s.take(1), s.drop(2)))
object Opt:
  def unapply(s: String): Option[String] = if s.isEmpty then None else Some(s)

// Nested: a top-level object's static forwarders come out of scalac's two Scala.js builds in another order.
object Bodies:
  object X:
    def pair(s: String): String = s match
      case Pair(a, b) => b + a
    def words(s: String): String = s match
      case Words(a, b) => b + a
      case _ => s
    def opt(s: String): String = s match
      case Opt(a) => a
      case _ => "none"
