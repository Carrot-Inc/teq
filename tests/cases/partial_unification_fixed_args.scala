// A higher-kinded variable unified with a class applied to more arguments takes the leftmost ones
// fixed (dotty's `canInstantiate`: `G[B] >: ZIO[R, Nothing, B]` makes `G` the `ZIO[R, Nothing, _]`
// of the class's own arguments). teq's variables for those arguments are instantiated to them, a
// `Nothing` too, so a `traverse` over an effect of no error, under a `flatMap` whose result has
// an error type, is that effect's (`ZIO[R, Nothing, _]`) and conforms; a mapped one as well.
final class ZIO[-R, +E, +A](val run: R => Either[E, A]):
  def flatMap[R1 <: R, E1 >: E, B](k: A => ZIO[R1, E1, B]): ZIO[R1, E1, B] =
    ZIO(r => run(r) match
      case Right(a) => k(a).run(r)
      case Left(e) => Left(e))
  def map[B](f: A => B): ZIO[R, E, B] = ZIO(r => run(r).map(f))
object ZIO:
  def succeed[A](a: => A): ZIO[Any, Nothing, A] = ZIO(_ => Right(a))
  def attempt[A](a: => A): ZIO[Any, Throwable, A] = ZIO(_ => try Right(a) catch case t: Throwable => Left(t))
trait Applicative[F[_]]:
  def pure[A](a: A): F[A]
  def map2[A, B, C](fa: F[A], fb: F[B])(f: (A, B) => C): F[C]
given zioApplicative[R, E]: Applicative[[A] =>> ZIO[R, E, A]] with
  def pure[A](a: A): ZIO[R, E, A] = ZIO(_ => Right(a))
  def map2[A, B, C](fa: ZIO[R, E, A], fb: ZIO[R, E, B])(f: (A, B) => C): ZIO[R, E, C] =
    fa.flatMap(a => fb.map(b => f(a, b)))
extension [A](as: List[A])
  def traverse[G[_], B](f: A => G[B])(using g: Applicative[G]): G[List[B]] =
    as.foldRight(g.pure(List.empty[B]))((a, acc) => g.map2(f(a), acc)(_ :: _))
var seen = List.empty[String]
def log(s: String): ZIO[Any, Nothing, Unit] = ZIO.succeed { seen = seen :+ s }

def logged(ids: List[Int]): ZIO[Any, Throwable, Unit] =
  ZIO.attempt(ids).flatMap(ys => ys.traverse(y => log("y" + y)).map(_ => ()))

def counted(ids: List[Int]): ZIO[Any, Throwable, Int] =
  for
    xs <- ZIO.attempt(ids)
    ls <- xs.traverse(x => log("x" + x))
    n <- ZIO.attempt(xs.sum)
  yield ls.size + n

@main def run(): Unit =
  println(logged(List(1, 2)).run(()))
  println(counted(List(3, 4)).run(()))
  println(seen)
