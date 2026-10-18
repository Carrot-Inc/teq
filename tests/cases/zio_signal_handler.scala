//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// os: unix
// split-watch: jvm
// zio 2.1.26's own `Platform.addSignalHandler`: `zio.internal.Signal` finds `sun.misc.Signal` and its handler
// interface by `Class.forName`, their constructor and `handle` by `MethodHandles.Lookup`, and installs a `Proxy`
// of the interface over its callback; the process sends itself SIGUSR1, whose handler prints.
package zio

object Main:
  def main(args: Array[String]): Unit =
    Unsafe.unsafe { implicit u =>
      internal.Platform.addSignalHandler("USR1", () => println("handled"))
    }
    println("ready")
    new ProcessBuilder("kill", "-USR1", ProcessHandle.current().pid().toString).start().waitFor()
    Thread.sleep(1500)
    println("done")
