// Givens whose type argument is a type lambda, found for lambdas written differently, for
// aliases of them and for constructors inferred by partial unification.
package demo.lambdas

trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]

trait Applicative[F[_]] extends Functor[F]:
  def pure[A](a: A): F[A]
  def map2[A, B, C](fa: F[A], fb: F[B])(f: (A, B) => C): F[C]
  def map[A, B](fa: F[A])(f: A => B): F[B] = map2(fa, pure(()))((a, _) => f(a))

sealed trait Validated[+E, +A]
final case class Valid[A](a: A) extends Validated[Nothing, A]
final case class Invalid[E](e: E) extends Validated[E, Nothing]

object Validated:
  given [E]: Applicative[[X] =>> Validated[List[E], X]] with
    def pure[A](a: A): Validated[List[E], A] = Valid(a)
    def map2[A, B, C](fa: Validated[List[E], A], fb: Validated[List[E], B])(f: (A, B) => C): Validated[List[E], C] =
      (fa, fb) match
        case (Valid(a), Valid(b)) => Valid(f(a, b))
        case (Invalid(e1), Invalid(e2)) => Invalid(e1 ++ e2)
        case (Invalid(e), _) => Invalid(e)
        case (_, Invalid(e)) => Invalid(e)

final case class Failure(msg: String)

final case class ZIO[-R, +E, +A](run: R => Either[E, A])
type Task[+A] = ZIO[Any, Failure, A]
type UIO[+A] = ZIO[Any, Nothing, A]
type IO[+E, +A] = ZIO[Any, E, A]

object ZIO:
  def succeed[A](a: A): UIO[A] = ZIO(_ => Right(a))
  given [R, E]: Applicative[[A] =>> ZIO[R, E, A]] with
    def pure[A](a: A): ZIO[R, E, A] = ZIO(_ => Right(a))
    def map2[A, B, C](fa: ZIO[R, E, A], fb: ZIO[R, E, B])(f: (A, B) => C): ZIO[R, E, C] =
      ZIO(r => fa.run(r).flatMap(a => fb.run(r).map(b => f(a, b))))

given Applicative[Option] with
  def pure[A](a: A): Option[A] = Some(a)
  def map2[A, B, C](fa: Option[A], fb: Option[B])(f: (A, B) => C): Option[C] =
    fa.flatMap(a => fb.map(b => f(a, b)))

given eitherApplicative[L]: Applicative[[X] =>> Either[L, X]] with
  def pure[A](a: A): Either[L, A] = Right(a)
  def map2[A, B, C](fa: Either[L, A], fb: Either[L, B])(f: (A, B) => C): Either[L, C] =
    fa.flatMap(a => fb.map(b => f(a, b)))

extension [A](xs: List[A])
  def traverse[G[_], B](f: A => G[B])(using ap: Applicative[G]): G[List[B]] =
    xs.foldRight(ap.pure(List.empty[B]))((a, acc) => ap.map2(f(a), acc)(_ :: _))

def pureOf[F[_]](n: Int)(using ap: Applicative[F]): F[Int] = ap.pure(n)

def positive(n: Int): Validated[List[String], Int] =
  if n > 0 then Valid(n) else Invalid(List(n.toString + " is not positive"))

def task(n: Int): Task[Int] = ZIO(_ => Right(n * 2))
def failing(n: Int): IO[String, Int] =
  val result: Either[String, Int] = if n > 1 then Left("too big: " + n) else Right(n)
  ZIO(_ => result)
def half(n: Int): Either[String, Int] = if n % 2 == 0 then Right(n / 2) else Left("odd: " + n)

@main def main(): Unit =
  println(List(1, 2, 3).traverse(positive))
  println(List(1, -2, -3).traverse(positive))
  println(List(1, 2, 3).traverse(task).run(()))
  println(List(1, 2, 3).traverse(failing).run(()))
  println(List(2, 4).traverse(half))
  println(List(2, 3).traverse(half))
  println(List(1, 2).traverse(n => Option(n)))
  println(pureOf[Task](5).run(()))
  println(pureOf[[X] =>> ZIO[Any, Failure, X]](6).run(()))
  println(pureOf[[X] =>> Either[String, X]](7))
  println(pureOf[Option](8))
  println(summon[Applicative[[T] =>> Validated[List[Int], T]]].pure(9))
  println(summon[Functor[UIO]].map(ZIO.succeed(1))(_ + 1).run(()))
