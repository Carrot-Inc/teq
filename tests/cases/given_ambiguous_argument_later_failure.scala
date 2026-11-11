trait Monad[F[_]]
object Monad:
  given Monad[List] = ???
  given Monad[Vector] = ???
trait Distribution[F[_], -P, X] extends (P => F[X])
trait P2[+A, +B]
trait Shrink[T]
trait Buildable[T]
object Shrink extends Low:
  implicit def container[C[_], T](implicit v: C[T] => Iterable[T], b: Buildable[T]): Shrink[C[T]] = ???
trait Low:
  implicit def any[T]: Shrink[T] = new Shrink[T] {}
given d: [F[_]: Monad] => Distribution[F, P2[Double, Double], Double] = ???
@main def run(): Unit = println(summon[Shrink[P2[Double, Double]]] != null)
