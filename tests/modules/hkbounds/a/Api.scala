package ha

trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]

object Functor:
  given Functor[List] with
    def map[A, B](fa: List[A])(f: A => B): List[B] = fa.map(f)
  given Functor[Option] with
    def map[A, B](fa: Option[A])(f: A => B): Option[B] = fa.map(f)

def lift[F[_]: Functor, A, B](fa: F[A])(f: A => B): F[B] = summon[Functor[F]].map(fa)(f)
def largest[A: Ordering](xs: List[A]): A = xs.max
