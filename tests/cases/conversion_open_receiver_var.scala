import scala.language.implicitConversions

// A conversion that gives a member its receiver leaves a variable its result holds nonvariantly
// open (dotty's `interpolateTypeVars`: no interpolation of a nonvariant variable), for the type the
// rest of the expression meets to fix: cats' `NoFile("..").pure[F]` against an expected
// `F[Result[A]]` takes `A` of `ApplicativeIdOps[A]` as `Result[A]`, as tyrian's file reader does.
trait Applicative[F[_]]:
  def pure[A](a: A): F[A]
given Applicative[List] with
  def pure[A](a: A): List[A] = List(a)
given Applicative[Option] with
  def pure[A](a: A): Option[A] = Some(a)

final class IdOps[A](private val a: A) extends AnyVal:
  def pure[F[_]](implicit F: Applicative[F]): F[A] = F.pure(a)
implicit def idOps[A](a: A): IdOps[A] = new IdOps(a)

sealed trait Result[+A]
case class NoFile(msg: String) extends Result[Nothing]
case class Found[A](a: A) extends Result[A]

def results[F[_]: Applicative](b: Boolean): F[Result[Int]] =
  if b then NoFile("none").pure[F] else Found(1).pure[F]

def takeOption[F[_]](x: F[Option[Int]]): String = x.toString

@main def run(): Unit =
  println(results[List](true))
  println(results[Option](false))
  println(takeOption[List](Some(1).pure[List]))
  val widened: List[Option[Int]] = Some(2).pure[List]
  println(widened)
  val plain = Some(3).pure[List]
  println(plain.map(_.get + 1))
