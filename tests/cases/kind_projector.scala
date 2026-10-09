// teq: --kind-projector
//> using options -Xkind-projector
// scalac's -Xkind-projector: a `*` among a type's arguments makes it a type lambda.
trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]

given eitherFunctor[L]: Functor[Either[L, *]] with
  def map[A, B](fa: Either[L, A])(f: A => B): Either[L, B] = fa.map(f)

def twice[F[_]](fa: F[Int])(using F: Functor[F]): F[Int] = F.map(fa)(_ * 2)

type Keyed[K] = Map[K, *]

@main def run(): Unit =
  println(twice[Either[String, *]](Right(21)))
  println(twice[Either[String, *]](Left("no")))
  val m: Keyed[String][Int] = Map("a" -> 1)
  println(m)
