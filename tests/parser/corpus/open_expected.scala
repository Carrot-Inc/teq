opaque type Refined[T, P] = T
object Refined:
  def unsafeApply[T, P](t: T): Refined[T, P] = t

trait Validate[T, P]:
  def isValid(t: T): Boolean

final class RefinedType[FTP, T](check: T => Either[String, FTP]):
  def refine(t: T): Either[String, FTP] = check(t)

object RefinedType:
  def of[T, P](using v: Validate[T, P]): RefinedType[Refined[T, P], T] =
    RefinedType(t => if v.isValid(t) then Right(Refined.unsafeApply(t)) else Left("invalid"))

trait Positive
given Validate[Int, Positive] with
  def isValid(t: Int): Boolean = t > 0

@main def run(): Unit =
  val rt = RefinedType.of[Int, Positive]
  println(rt.refine(3).isRight)
  println(rt.refine(-1))
  val direct: Either[String, Refined[Int, Positive]] = Right(Refined.unsafeApply(7))
  println(direct.isRight)
