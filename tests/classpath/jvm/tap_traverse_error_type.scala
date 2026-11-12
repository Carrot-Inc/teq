// jars: scala-library zio-jvm zio-interop-cats-jvm zio-streams-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm cats-core cats-kernel cats-effect-jvm cats-effect-kernel-jvm cats-effect-std-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm zio-interop-tracer-jvm
// std: scala-library
// targets: jvm
//> using dep dev.zio::zio:2.1.26
//> using dep dev.zio::zio-interop-cats:23.1.0.13
//> using dep org.typelevel::cats-core:2.13.0
// zio's `tap` over cats' `traverse_`: the lambda's body is typed against `ZIO[R1, E1, Any]` with both variables
// open, and `traverse_`'s `G` is the constructor of the argument's type, `ZIO[Any, Nothing, _]`, as dotty minimizes
// it before the `Applicative` is searched: zio-interop-cats' general instance answers, not the one for
// `Throwable`, and the error type of `tap`'s result stays `Nothing`.
import zio.*
import zio.interop.catz.*
import cats.syntax.all.*

object Probe:
  def serve(n: Int): UIO[Unit] = ZIO.unit
  def next: UIO[Option[Int]] = ZIO.succeed(Some(1))
  def d: UIO[Option[Int]] =
    val t = next.tap(_.traverse_(serve))
    t
  def e: UIO[Option[Int]] =
    val t = next.tap(o => o.traverse_(serve))
    t
  def f: UIO[Option[Int]] =
    val t = next.tap(o => (o.traverse_(serve): UIO[Unit]))
    t
  def g: UIO[Unit] =
    val u = Option(1).traverse_(serve)
    u
  def h: UIO[Option[Int]] =
    val t = next.tap(o => ZIO.foreachDiscard(o)(serve))
    t

@main def run(): Unit =
  def run[A](io: UIO[A]): A = Unsafe.unsafe { implicit u => Runtime.default.unsafe.run(io).getOrThrowFiberFailure() }
  println(run(Probe.d))
  println(run(Probe.e))
  println(run(Probe.f))
  println(run(Probe.g))
  println(run(Probe.h))
