// An upper-bounded type variable (`R1 <: R` of `flatMap`) stays open while the argument is typed,
// so a nested application settles it: `Any` is not chosen for `R1` before the inner effect asks
// for `WorldState`. A generator whose element type is `Nothing` is accepted.
final case class Eff[-R, +E, +A](run: R => Either[E, A]):
  def flatMap[R1 <: R, E1 >: E, B](f: A => Eff[R1, E1, B]): Eff[R1, E1, B] =
    Eff(r => run(r).fold(e => Left(e), a => f(a).run(r)))
  def map[B](f: A => B): Eff[R, E, B] = Eff(r => run(r).map(f))
  def catchAll[R1 <: R, E2, A1 >: A](h: E => Eff[R1, E2, A1]): Eff[R1, E2, A1] =
    Eff(r => run(r).fold(e => h(e).run(r), a => Right(a)))
  def tapError[R1 <: R, E1 >: E](f: E => Eff[R1, E1, Any]): Eff[R1, E1, A] =
    Eff(r => run(r).fold(e => f(e).run(r).flatMap(_ => Left(e)), a => Right(a)))
  def *>[R1 <: R, E1 >: E, B](that: Eff[R1, E1, B]): Eff[R1, E1, B] = flatMap(_ => that)

object Eff:
  def succeed[A](a: => A): Eff[Any, Nothing, A] = Eff(_ => Right(a))
  def fail[E](e: => E): Eff[Any, E, Nothing] = Eff(_ => Left(e))
  def service[R]: Eff[R, Nothing, R] = Eff(r => Right(r))
  def attempt[A](a: => A): Eff[Any, String, A] = Eff(_ => Right(a))

type Task[A] = Eff[Any, String, A]
type UIO[A] = Eff[Any, Nothing, A]
final case class WorldState(n: Int)
type AppTask[A] = Eff[WorldState, String, A]

object Infer:
  val task: Task[Unit] = Eff.attempt(())
  val uio: UIO[Int] = Eff.succeed(5)
  val app: AppTask[Int] = Eff.service[WorldState].map(_.n)
  val failing: Task[Int] = Eff.fail("boom")

  val t3 = task.flatMap(_ => app.flatMap(r => task.map(_ => r)))
  val receiver = (for
    _ <- task
    r <- app
    _ <- task
  yield r).catchAll(_ => Eff.succeed(0))
  val five = for
    _ <- task
    a <- app
    b <- uio
    _ <- task
    c <- app
  yield a + b + c
  val lowerBound = uio.flatMap(x => task.flatMap(_ => app.map(_ + x)))
  val recovered = (for
    _ <- task
    r <- failing
    s <- app
  yield r + s).catchAll(msg => Eff.succeed(msg.length))
  val tapped = (for
    _ <- task
    r <- app
  yield r).tapError(e => Eff.succeed(println(e)))
  val nothingGen = for
    u <- Eff.succeed(1).flatMap(_ => Eff.fail("bad"))
  yield u
  val sequenced = task *> app *> uio

@main def main(): Unit =
  val env = WorldState(7)
  println(Infer.t3.run(env))
  println(Infer.receiver.run(env))
  println(Infer.five.run(env))
  println(Infer.lowerBound.run(env))
  println(Infer.recovered.run(env))
  println(Infer.tapped.run(env))
  println(Infer.nothingGen.run(()))
  println(Infer.sequenced.run(env))
