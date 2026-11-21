package kbx

class Mode(val v: Int)
object Mode:
  given fallback: Mode = Mode(101)

// An extractor whose using clause is searched where the library's body is: the given chosen there (imported) is
// the one the pickle holds; over the library's products the pattern takes it, not the companion's fallback nor a
// given of the user's.
object E:
  def unapply(x: Int)(using m: Mode): Option[Int] = Some(x + m.v)
object Good:
  given chosen: Mode = Mode(7)

object Lib:
  import Good.given
  def value: Int = 1 match { case E(n) => n }
