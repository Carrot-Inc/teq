// The branches of an `if` or `match` are joined before the result meets an expected type that
// applies an unsolved type constructor variable, so `Some(x)` and `None` make `G` an `Option`.
trait Applicative[F[_]]:
  def pure[A](a: A): F[A]
  extension [A](fa: F[A]) def map[B](f: A => B): F[B]
  extension [A](fa: F[A]) def map2[B, C](fb: F[B])(f: (A, B) => C): F[C]

given Applicative[Option] with
  def pure[A](a: A): Option[A] = Some(a)
  extension [A](fa: Option[A]) def map[B](f: A => B): Option[B] = fa match
    case Some(a) => Some(f(a))
    case None => None
  extension [A](fa: Option[A]) def map2[B, C](fb: Option[B])(f: (A, B) => C): Option[C] = (fa, fb) match
    case (Some(a), Some(b)) => Some(f(a, b))
    case _ => None

given [E]: Applicative[[X] =>> Either[E, X]] with
  def pure[A](a: A): Either[E, A] = Right(a)
  extension [A](fa: Either[E, A]) def map[B](f: A => B): Either[E, B] = fa match
    case Right(a) => Right(f(a))
    case Left(e) => Left(e)
  extension [A](fa: Either[E, A]) def map2[B, C](fb: Either[E, B])(f: (A, B) => C): Either[E, C] = (fa, fb) match
    case (Right(a), Right(b)) => Right(f(a, b))
    case (Left(e), _) => Left(e)
    case (_, Left(e)) => Left(e)

def traverse[G[_], A, B](xs: List[A])(f: A => G[B])(using G: Applicative[G]): G[List[B]] =
  xs.foldRight(G.pure(List.empty[B]))((a, acc) => G.map2(f(a))(acc)((b, bs) => b :: bs))

@main def main(): Unit =
  println(traverse(List(1, 2))(x => if x > 0 then Some(x) else None))
  println(traverse(List(0, 2))(x => if x > 0 then Some(x) else None))
  println(traverse(List(1, 2))(x => x match
    case 0 => None
    case n => Some(n * 10)))
  println(traverse(List("a", "bb"))(s => if s.length > 1 then Right(s) else Left(s"short: $s")))
  println(traverse(List("aa", "bb"))(s => if s.length > 1 then Right(s.toUpperCase) else Left(s)))
  println(traverse(List(3))(x => x match
    case 3 => Left("three")
    case _ => Right(x)))
