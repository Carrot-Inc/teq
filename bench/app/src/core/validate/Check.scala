package meridian.core.validate

/** A validation that collects every fault instead of stopping at the first. */
final case class Fault(path: String, message: String):
  def render: String = if path.isEmpty then message else s"$path: $message"
object Fault:
  given cats.Eq[Fault] = cats.Eq.fromUniversalEquals
  given cats.Show[Fault] = f => f.render

enum Check[+A]:
  case Valid(value: A)
  case Invalid(errors: List[Fault])

  def map[B](f: A => B): Check[B] = this match
    case Valid(a) => Valid(f(a))
    case Invalid(fs) => Invalid(fs)
  def andThen[B](f: A => Check[B]): Check[B] = this match
    case Valid(a) => f(a)
    case Invalid(fs) => Invalid(fs)
  def zip[B](that: Check[B]): Check[(A, B)] = (this, that) match
    case (Valid(a), Valid(b)) => Valid((a, b))
    case (Invalid(x), Invalid(y)) => Invalid(x ++ y)
    case (Invalid(x), _) => Invalid(x)
    case (_, Invalid(y)) => Invalid(y)
  def as[B](b: B): Check[B] = map(_ => b)
  def void: Check[Unit] = map(_ => ())
  def isValid: Boolean = this match
    case Valid(_) => true
    case Invalid(_) => false
  def isInvalid: Boolean = !isValid
  def toEither: Either[List[Fault], A] = this match
    case Valid(a) => Right(a)
    case Invalid(fs) => Left(fs)
  def toOption: Option[A] = toEither.toOption
  def faults: List[Fault] = this match
    case Valid(_) => Nil
    case Invalid(fs) => fs
  def at(path: String): Check[A] = this match
    case Invalid(fs) => Invalid(fs.map(f => f.copy(path = if f.path.isEmpty then path else s"$path.${f.path}")))
    case valid => valid
  def orElse[A1 >: A](that: => Check[A1]): Check[A1] = this match
    case Invalid(_) => that
    case valid => valid

object Check:
  def valid[A](a: A): Check[A] = Valid(a)
  def invalid[A](message: String): Check[A] = Invalid(List(Fault("", message)))
  def cond[A](test: Boolean, value: => A, message: => String): Check[A] = if test then Valid(value) else invalid(message)
  def fromOption[A](option: Option[A], message: => String): Check[A] = option.fold(invalid[A](message))(Valid(_))
  def fromEither[A](either: Either[String, A]): Check[A] = either.fold(invalid[A], Valid(_))
  def all[A](checks: List[Check[A]]): Check[List[A]] =
    checks.foldRight[Check[List[A]]](Valid(Nil))((c, acc) => c.zip(acc).map((a, as) => a :: as))
  def traverse[A, B](as: List[A])(f: A => Check[B]): Check[List[B]] = all(as.map(f))
  def map2[A, B, C](a: Check[A], b: Check[B])(f: (A, B) => C): Check[C] = a.zip(b).map(f.tupled)
  def map3[A, B, C, D](a: Check[A], b: Check[B], c: Check[C])(f: (A, B, C) => D): Check[D] =
    a.zip(b).zip(c).map { case ((x, y), z) => f(x, y, z) }
  def map4[A, B, C, D, E](a: Check[A], b: Check[B], c: Check[C], d: Check[D])(f: (A, B, C, D) => E): Check[E] =
    a.zip(b).zip(c).zip(d).map { case (((x, y), z), w) => f(x, y, z, w) }
  def map5[A, B, C, D, E, F](a: Check[A], b: Check[B], c: Check[C], d: Check[D], e: Check[E])(f: (A, B, C, D, E) => F): Check[F] =
    a.zip(b).zip(c).zip(d).zip(e).map { case ((((x, y), z), w), v) => f(x, y, z, w, v) }

extension [A](a: A)
  def validCheck: Check[A] = Check.Valid(a)
extension (message: String)
  def invalidCheck[A]: Check[A] = Check.invalid(message)
extension [A](option: Option[A])
  def toCheck(message: => String): Check[A] = Check.fromOption(option, message)

object Rules:
  def nonEmpty(field: String, value: String): Check[String] = Check.cond(value.trim.nonEmpty, value.trim, s"$field must not be empty")
  def maxLength(field: String, value: String, limit: Int): Check[String] = Check.cond(value.length <= limit, value, s"$field is longer than $limit")
  def positive(field: String, value: Long): Check[Long] = Check.cond(value > 0, value, s"$field must be positive")
  def nonNegative(field: String, value: Long): Check[Long] = Check.cond(value >= 0, value, s"$field must not be negative")
  def within(field: String, value: Int, low: Int, high: Int): Check[Int] = Check.cond(value >= low && value <= high, value, s"$field must be between $low and $high")
  def oneOf[A](field: String, value: A, allowed: List[A]): Check[A] = Check.cond(allowed.contains(value), value, s"$field is not allowed")
  def defined[A](field: String, value: Option[A]): Check[A] = Check.fromOption(value, s"$field is required")
