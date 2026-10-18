//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// Two blocking tasks read one lazy val, the second while the first evaluates it: the body runs once.
import zio.*
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicInteger
class SharedBox(ready: CountDownLatch) {
 val calls = new AtomicInteger()
 lazy val value: Int = { calls.incrementAndGet(); ready.countDown(); Thread.sleep(1000); 42 }
}
object Main extends ZIOAppDefault {
 def run = for {
  ready <- ZIO.succeed(new CountDownLatch(1))
  box <- ZIO.succeed(new SharedBox(ready))
  f <- ZIO.attemptBlocking(box.value).fork
  _ <- ZIO.attemptBlocking(ready.await())
  second <- ZIO.attemptBlocking(box.value)
  first <- f.join
  _ <- Console.printLine(s"$first,$second,${box.calls.get()}")
 } yield ()
}
