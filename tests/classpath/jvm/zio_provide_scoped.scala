// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm
// std: scala-library
//> using dep dev.zio::zio:2.1.26
// `provide` over a chain of `++` with a scoped layer, the application's integration tests'
// shape: the chain is a `ZLayer[Any, ..]` (its input variable maximised, as scalac does), and
// the layer macro's run over it, which uses zio's `Chunk`, ends.
import zio.*

final case class A(v: Int)
final case class B(v: String)
final case class C(v: Long)
final case class D(v: Double)

object Main extends ZIOAppDefault:
  def mkB: ZIO[Scope, Throwable, B] = ZIO.acquireRelease(ZIO.succeed(B("b")))(_ => ZIO.succeed(println("released")))
  val a: ULayer[A] = ZLayer.succeed(A(1))
  val d: TaskLayer[D] = ZLayer.succeed(D(1.5))
  val env = a ++ ZLayer.scoped(mkB) ++ ZLayer.succeed(C(3L)) ++ d

  def run =
    val program =
      for
        a <- ZIO.service[A]
        b <- ZIO.service[B]
        c <- ZIO.service[C]
        d <- ZIO.service[D]
      yield println(s"${a.v} ${b.v} ${c.v} ${d.v}")
    program.provide(env)
