//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// A blocking read on a pipe that another fiber completes: a fiber reads a `PipedInputStream` in
// `attemptBlocking` while another writes after a sleep; then a read that another fiber cancels,
// `attemptBlockingInterrupt` interrupting the blocked thread, whose read ends in
// `InterruptedIOException`.
import java.io.{PipedInputStream, PipedOutputStream}
import zio.*

object Main extends ZIOAppDefault:
  def run =
    for
      in <- ZIO.succeed(new PipedInputStream())
      out <- ZIO.succeed(new PipedOutputStream(in))
      reader <- ZIO.attemptBlocking(in.read()).fork
      _ <- (ZIO.sleep(30.millis) *> ZIO.attemptBlocking { out.write(42); out.flush() }).fork
      b <- reader.join
      _ <- Console.printLine("read " + b)
      blocked <- ZIO.attemptBlockingInterrupt(in.read()).either.fork
      _ <- ZIO.sleep(30.millis)
      exit <- blocked.interrupt
      _ <- Console.printLine("cancelled read interrupted: " + exit.isInterrupted)
      _ <- ZIO.attemptBlocking(out.close())
      eof <- ZIO.attemptBlocking(in.read())
      _ <- Console.printLine("after close " + eof)
    yield ()
