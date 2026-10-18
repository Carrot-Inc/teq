//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// Two blocking tasks read one object, the second while the first initialises it: both see it complete.
import zio.*
import java.util.concurrent.CountDownLatch
object Gate { val entered = new CountDownLatch(1) }
object Slow { Gate.entered.countDown(); Thread.sleep(1000); val value = "ready" }
object Main extends ZIOAppDefault {
 def run = for {
  ready <- ZIO.succeed(Gate.entered)
  f <- ZIO.attemptBlocking(Slow.value).fork
  _ <- ZIO.attemptBlocking(ready.await())
  second <- ZIO.attemptBlocking(Slow.value)
  first <- f.join
  _ <- Console.printLine(s"$first,$second")
 } yield ()
}
