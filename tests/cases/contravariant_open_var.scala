// A type variable with upper bounds only that the result uses contravariantly is maximised, also
// when the lambda's result comes through a given that unifies its own variable with it: `R1` of
// `flatMap[R1 <: R]` stays `Any` for a `_.traverse_(set)` body, as under scalac.

final class F[-R, +A](val run: R => A):
  def flatMap[R1 <: R, B](f: A => F[R1, B]): F[R1, B] = new F[R1, B](r => f(run(r)).run(r))
  def map[B](f: A => B): F[R, B] = new F[R, B](r => f(run(r)))

trait Applicative[G[_]]:
  def pure[A](a: A): G[A]

object Applicative:
  given [R]: Applicative[[X] =>> F[R, X]] = new Applicative[[X] =>> F[R, X]]:
    def pure[A](a: A): F[R, A] = new F[R, A](_ => a)

extension [A](o: Option[A])
  def traverse_[G[_], B](f: A => G[B])(using G: Applicative[G]): G[Unit] = o match
    case Some(a) => G.pure(())
    case None => G.pure(())

val set: Boolean => F[Any, Unit] = _ => new F[Any, Unit](_ => ())
val z: F[Any, Option[Boolean]] = new F[Any, Option[Boolean]](_ => Some(true))

@main def main(): Unit =
  val w = z.flatMap(_.traverse_(set))
  val ok: F[Any, Unit] = w
  println(ok.map(_ => "done").run(()))
  val v = z.flatMap(o => new F[Any, Int](_ => o.size))
  val ok2: F[Any, Int] = v
  println(ok2.run(()))
