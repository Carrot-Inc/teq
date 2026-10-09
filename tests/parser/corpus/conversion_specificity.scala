// Two implicit conversions provide the same extension method: the one whose parameter type the
// other's conforms to is the more specific (SLS 6.26.3), so a conversion from `Either[A, B]`
// beats one from `F[A, B]` for any `F`, however the type parameters would be inferred.
trait Bifunctor[F[_, _]]:
  def bimap[A, B, C, D](fab: F[A, B])(f: A => C, g: B => D): F[C, D]

given Bifunctor[Either] with
  def bimap[A, B, C, D](fab: Either[A, B])(f: A => C, g: B => D): Either[C, D] = fab match
    case Left(a) => Left(f(a))
    case Right(b) => Right(g(b))

final class BifunctorOps[F[_, _], A, B](fab: F[A, B])(using bf: Bifunctor[F]):
  def leftMap[C](f: A => C): F[C, B] = { println("via Bifunctor"); bf.bimap(fab)(f, identity) }
  def rightMap[D](g: B => D): F[A, D] = bf.bimap(fab)(identity, g)

final class EitherOps[A, B](eab: Either[A, B]):
  def leftMap[C](f: A => C): Either[C, B] = { println("via Either"); eab.left.map(f) }
  def leftWiden[AA >: A]: Either[AA, B] = eab

object syntax:
  implicit def toBifunctorOps[F[_, _], A, B](fab: F[A, B])(using Bifunctor[F]): BifunctorOps[F, A, B] = new BifunctorOps(fab)
  implicit def toEitherOps[A, B](eab: Either[A, B]): EitherOps[A, B] = new EitherOps(eab)

@main def run(): Unit =
  import syntax.*
  val e: Either[String, Int] = Left("no")
  println(e.leftMap(_.length))
  println(Right(3).leftMap((s: String) => s.length))
  println(e.rightMap(_ + 1))
  println(e.leftWiden[Any])
