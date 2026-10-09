// Adapted from scala3 tests/neg/i24914.scala (Apache-2.0, see tests/scala3/README.md): a forest of sibling givens of one signature.
// expect: 27:19: error: no given instance of type Functor[F] was found for parameter evidence$
// expect: 1 error found
trait Functor[F[_]]
trait Monad[F[_]] extends Functor[F]

class T[F[_], A, X]

object Functor:
  given t1[F[_]: Functor, A]: Functor[[X] =>> T[F, A, X]] = ???
  given t2[F[_]: Functor, A]: Functor[[X] =>> T[F, A, X]] = ???
  given t3[F[_]: Functor, A]: Functor[[X] =>> T[F, A, X]] = ???
  given t4[F[_]: Monad, A]: Functor[[X] =>> T[F, A, X]] = ???
  given t5[F[_]: Functor, A]: Functor[[X] =>> T[F, A, X]] = ???
  given t6[F[_]: Functor, A]: Functor[[X] =>> T[F, A, X]] = ???

object Monad:
  given t1[F[_]: Monad, A]: Monad[[X] =>> T[F, A, X]] = ???
  given t2[F[_]: Monad, A]: Monad[[X] =>> T[F, A, X]] = ???
  given t3[F[_]: Monad, A]: Monad[[X] =>> T[F, A, X]] = ???
  given t4[F[_]: Monad, A]: Monad[[X] =>> T[F, A, X]] = ???
  given t5[F[_]: Monad, A]: Monad[[X] =>> T[F, A, X]] = ???
  given t6[F[_]: Monad, A]: Monad[[X] =>> T[F, A, X]] = ???

def m[F[_]: Functor]: F[String] = ???

val x = for _ <- m yield ()
