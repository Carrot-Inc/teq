package meridian.core.refined

/** Values with a proven property, checked once at the boundary. */
opaque type PosInt = Int
object PosInt:
  def from(n: Int): Either[String, PosInt] = if n > 0 then Right(n) else Left(s"Predicate failed: ($n > 0).")
  def unsafeFrom(n: Int): PosInt = from(n).fold(e => throw new IllegalArgumentException(e), identity)
  def unapply(n: Int): Option[PosInt] = from(n).toOption
  extension (p: PosInt) def value: Int = p

opaque type NonNegInt = Int
object NonNegInt:
  def from(n: Int): Either[String, NonNegInt] = if n >= 0 then Right(n) else Left(s"Predicate ($n < 0) did not fail.")
  def unsafeFrom(n: Int): NonNegInt = from(n).fold(e => throw new IllegalArgumentException(e), identity)
  def unapply(n: Int): Option[NonNegInt] = from(n).toOption
  val zero: NonNegInt = 0
  extension (p: NonNegInt) def value: Int = p

opaque type NonEmptyText = String
object NonEmptyText:
  def from(s: String): Either[String, NonEmptyText] = if s.nonEmpty then Right(s) else Left("Predicate isEmpty() did not fail.")
  def unsafeFrom(s: String): NonEmptyText = from(s).fold(e => throw new IllegalArgumentException(e), identity)
  def unapply(s: String): Option[NonEmptyText] = from(s).toOption
  extension (t: NonEmptyText) def value: String = t

opaque type Percent = Int
object Percent:
  def from(n: Int): Either[String, Percent] = if n >= 0 && n <= 100 then Right(n) else Left(s"Predicate failed: (0 <= $n <= 100).")
  def unsafeFrom(n: Int): Percent = from(n).fold(e => throw new IllegalArgumentException(e), identity)
  extension (p: Percent) def value: Int = p
  extension (p: Percent) def of(total: Long): Long = total * p / 100
