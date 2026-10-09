// The branches of an `if` whose results hold a type argument invariantly, as a free monad's
// `pure` does: scalac leaves the argument open where the result holds it invariantly, so that
// `pure(Left(e))` and `pure(Right(r))` join to a `Fr[Either[E, R]]` rather than to the
// `Product & Serializable` two different `Fr` applications share, and against an expected
// `Fr[B]` the first branch does not fix `B`.
sealed abstract class Fr[S[_], A] extends Product with Serializable:
  def map[B](f: A => B): Fr[S, B] = flatMap(a => Fr.Pure(f(a)))
  def flatMap[B](f: A => Fr[S, B]): Fr[S, B] = Fr.Bind(this, f)
object Fr:
  final case class Pure[S[_], A](a: A) extends Fr[S, A]
  final case class Bind[S[_], X, A](fa: Fr[S, X], f: X => Fr[S, A]) extends Fr[S, A]

trait Op[A]
type IO[A] = Fr[Op, A]

trait Sync[F[_]]:
  def pure[A](a: A): F[A]
object Sync:
  def apply[F[_]](using s: Sync[F]): Sync[F] = s
given Sync[IO] with
  def pure[A](a: A): IO[A] = Fr.Pure(a)

def pure[A](a: A): IO[A] = Fr.Pure(a)

def run[A](io: IO[A]): A = io match
  case Fr.Pure(a) => a
  case Fr.Bind(fa, f) => run(f(run(fa)))

def check(b: Boolean): IO[Either[String, Int]] = for
  ok     <- Sync[IO].pure(b)
  result <-
    if !ok then Sync[IO].pure(Left("no"))
    else for
      n <- Sync[IO].pure(41)
      _ <- pure(())
    yield Right(n + 1)
yield result

def either[B](f: Boolean => IO[B]): IO[B] = f(false)
def both[B](x: IO[B], y: IO[B]): IO[B] = y

@main def main(args: String*): Unit =
  println(run(check(true)))
  println(run(check(false)))
  println(run(either(ok => if !ok then pure(Left[String, Int]("no")) else pure(Right(1)))))
  println(run(either(ok => if !ok then pure(Right(1)) else pure(Left("no")))))
  println(run(both(pure(Left("no")), pure(Right(1)))))
  val c = args.nonEmpty
  val joined = if c then pure(Left("no")) else pure(Right(1))
  println(run(joined.map(_.isRight)))
  println(run((if c then pure(Left("no")) else pure(Right(2))).map(_.fold(_.length, _ + 1))))
  invariantResult()

class Inv[A](x: A):
  def get: A = x
object Inv:
  def empty[A]: Inv[A] = new Inv(null.asInstanceOf[A])

// An inferred result type settles what the branches left open: an `Inv[Int]`, not an `Inv[?A]`.
def inv(cond: Boolean) =
  if cond then new Inv(1)
  else Inv.empty

def invariantResult(): Unit =
  val i: Inv[Int] = inv(true)
  println(i.get)
