// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time zio-streams zio-managed zio-interop-cats zio-interop-tracer cats-kernel-sjs cats-core-sjs cats-effect-kernel cats-effect-std cats-effect cats-mtl fs2-core
//> using dep dev.zio::zio:2.1.26
//> using dep dev.zio::zio-streams:2.1.26
//> using dep dev.zio::zio-managed:2.1.26
//> using dep dev.zio::zio-interop-cats:23.1.0.13
// zio-interop-cats 23.1.0.13: `import zio.interop.catz.*` gives cats-effect's `Async`,
// `Concurrent` and `Temporal` for `Task`, which methods written against the cats type classes
// then run on: `Sync.delay`, `Ref.of` through `Concurrent`, `Temporal.sleep`, `parTupled`-style
// `both`, `MonadError` handlers.
package ziocats

import cats.effect.kernel.{Async, Concurrent, Ref as CRef, Sync, Temporal}
import cats.syntax.all.*
import zio.*
import zio.interop.catz.*
import scala.concurrent.duration.*

object Main extends ZIOAppDefault:
  def delayed[F[_]: Sync](n: Int): F[Int] = Sync[F].delay(n + 1)

  def counted[F[_]: Concurrent](xs: List[Int]): F[Int] =
    for
      r <- CRef.of[F, Int](0)
      _ <- xs.traverse_(x => r.update(_ + x))
      v <- r.get
    yield v

  def waited[F[_]: Temporal](label: String): F[String] =
    Temporal[F].sleep(5.millis) *> Temporal[F].pure(label)

  def recovered[F[_]: Async](s: String): F[Int] =
    Async[F].catchNonFatal(s.toInt).handleError(_ => -1)

  def paired[F[_]: Concurrent]: F[(Int, String)] =
    Concurrent[F].both(Concurrent[F].pure(1), Concurrent[F].pure("b"))

  def run =
    for
      a <- delayed[Task](1)
      b <- counted[Task](List(1, 2, 3))
      c <- waited[Task]("waited")
      d <- recovered[Task]("12")
      e <- recovered[Task]("x")
      f <- paired[Task]
      _ <- Console.printLine(s"$a $b $c $d $e $f")
    yield ()
