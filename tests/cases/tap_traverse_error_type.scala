// A lambda's result over an open error type: `tap`'s parameter result is `Z[R1, E1, Any]` with both variables open
// when the body `_.traverse_(serve)` is typed, and `traverse_`'s result is `G[Unit]`. dotty bounds `G` from above
// by the expected type, from below by the argument, and minimizes it before the implicit argument is searched
// (`Typer.adaptNoArgsImplicitMethod`, `Inferencing.instantiateSelected`): `G` is `[X] =>> Z[Any, Nothing, X]`, only
// the general instance fits, and the error type stays `Nothing`. scalac 3.8.4 compiles this and prints `ok`.
trait Applicative[F[_]]
final class Z[-R, +E, +A]:
  def tap[R1 <: R, E1 >: E](f: A => Z[R1, E1, Any]): Z[R1, E1, A] = Z()
type U[+A] = Z[Any, Nothing, A]
trait LowInstances:
  given generic[R, E]: Applicative[[X] =>> Z[R, E, X]] = new Applicative[[X] =>> Z[R, E, X]] {}
object Z extends LowInstances:
  given throwing[R]: Applicative[[X] =>> Z[R, Throwable, X]] = new Applicative[[X] =>> Z[R, Throwable, X]] {}
extension [F[_], A](fa: F[A])
  def traverse_[G[_], B](f: A => G[B])(using Applicative[G]): G[Unit] = ???
object Probe:
  def serve(n: Int): U[Unit] = Z()
  def next: U[Option[Int]] = Z()
  def d: U[Option[Int]] =
    val t = next.tap(_.traverse_(serve))
    t
  def e: U[Option[Int]] =
    val t = next.tap(o => o.traverse_(serve))
    t
  def f: U[Option[Int]] =
    val t = next.tap(o => (o.traverse_(serve): U[Unit]))
    t
  def g: U[Unit] =
    val u = Option(1).traverse_(serve)
    u
@main def run(): Unit = println("ok")
