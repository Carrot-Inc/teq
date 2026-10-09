// A class with fewer type arguments than a higher-kinded application conforms through the
// first base type with enough of them: `None` is an `F[A]` as `Option[Nothing]`, so a generic
// method over `F[_]` takes it (cats' `SemigroupK` syntax on `None`).
trait SK[F[_]]:
  def combineK[A](x: F[A], y: F[A]): F[A]
object SK:
  given SK[Option] with
    def combineK[A](x: Option[A], y: Option[A]): Option[A] = x.orElse(y)
  given SK[List] with
    def combineK[A](x: List[A], y: List[A]): List[A] = x ++ y
def widen[F[_], A](fa: F[A])(using F: SK[F]): F[A] = fa
def first[F[_], A](fa: F[A], fb: F[A])(using F: SK[F]): F[A] = F.combineK(fa, fb)
object Main:
  def main(args: Array[String]): Unit =
    println(widen(None))
    println(first(None, Option(2)))
    println(first(Nil, List(1)))
