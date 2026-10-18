//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// zio's own runtime under the JDK's threads: a `ZIOAppDefault` whose `run` prints a line, which
// starts `Runtime.default` (its `ZScheduler` and worker threads, the blocking executor), runs the
// effect on a worker, waits for it on the main thread and ends the program.
import zio.*

object Main extends ZIOAppDefault:
  def run = Console.printLine("hello from zio")
