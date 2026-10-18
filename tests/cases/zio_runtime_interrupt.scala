//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// Interruption with finalizers: a forked fiber sleeping for an hour interrupted by its parent runs its
// `ensuring` and `onInterrupt`; a race won by the faster sleeper interrupts the other; a timeout
// answers `None`; a scope's finalizers run in reverse order when it closes.
import zio.*

object Main extends ZIOAppDefault:
  def run =
    for
      f <- (ZIO.sleep(1.hour) *> Console.printLine("woke"))
             .onInterrupt(Console.printLine("interrupted").orDie)
             .ensuring(Console.printLine("finalizer ran").orDie)
             .fork
      _ <- ZIO.sleep(20.millis)
      exit <- f.interrupt
      _ <- Console.printLine("interrupted exit: " + exit.isInterrupted)
      winner <- ZIO.sleep(10.millis).as("fast").race(ZIO.sleep(1.second).as("slow").ensuring(Console.printLine("loser interrupted").orDie))
      _ <- Console.printLine("winner " + winner)
      t <- ZIO.sleep(1.second).as(1).timeout(20.millis)
      _ <- Console.printLine("timeout " + t)
      _ <- ZIO.scoped(
             ZIO.acquireRelease(Console.printLine("acquire a").as("a"))(_ => Console.printLine("release a").orDie) *>
               ZIO.acquireRelease(Console.printLine("acquire b").as("b"))(_ => Console.printLine("release b").orDie)
           )
    yield ()
