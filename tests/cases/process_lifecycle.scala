//> using platform jvm
// os: unix
// A child's life against the JDK: a timed wait that ends before the child, which stays alive; an
// exit status read before the end refused; ended by `destroy` (SIGTERM, 143) and `destroyForcibly`
// (SIGKILL, 137), the status kept for later reads; a child's own child left running when the child
// is killed, seen through `ProcessHandle` and ended by it; the children and descendants of a
// process; handles dropped and their children reaped all the same; the program's own handle; a
// missing program. The children are `sh` and `sleep`.
import java.util.concurrent.TimeUnit
import scala.jdk.CollectionConverters.*

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def main(args: Array[String]): Unit =
    val sleeper = new ProcessBuilder("sleep", "30").start()
    show("timed wait")(sleeper.waitFor(50, TimeUnit.MILLISECONDS))
    show("alive")(sleeper.isAlive())
    show("exitValue running")(sleeper.exitValue())
    show("toString running")(sleeper.toString.replace(sleeper.pid().toString, "<pid>"))
    show("handle")(sleeper.toHandle.pid() == sleeper.pid())
    show("handle alive")(sleeper.toHandle.isAlive())
    show("info command")(sleeper.info().command().map(c => c.endsWith("sleep")).orElse(false))
    show("info arguments")(sleeper.info().arguments().map(a => a.mkString(",")).orElse("none"))
    show("info command line")(sleeper.info().commandLine().map(c => c.endsWith("sleep 30")).orElse(false))
    show("among the children")(ProcessHandle.current().children().anyMatch(h => h.pid() == sleeper.pid()))
    sleeper.destroy()
    show("destroyed")(sleeper.waitFor())
    show("destroyed again")(sleeper.exitValue())
    show("toString ended")(sleeper.toString.replace(sleeper.pid().toString, "<pid>"))
    show("handle ended")(sleeper.toHandle.isAlive())
    val forced = new ProcessBuilder("sleep", "30").start()
    show("destroyForcibly")(forced.destroyForcibly() eq forced)
    show("forced")(forced.waitFor(10, TimeUnit.SECONDS))
    show("forced status")(forced.exitValue())
    show("forced again")(forced.waitFor())
    // A grandchild: the child prints its pid, then waits; killed, the child leaves it running.
    val parent = new ProcessBuilder("sh", "-c", "sleep 30 & echo $!; wait").start()
    val pid = new java.io.BufferedReader(new java.io.InputStreamReader(parent.getInputStream)).readLine().toLong
    show("deadline")(parent.waitFor(100, TimeUnit.MILLISECONDS))
    show("children")(parent.children().map(h => h.pid() == pid).toList.asScala.mkString(","))
    show("descendants")(ProcessHandle.current().descendants().anyMatch(h => h.pid() == pid))
    show("grandchild's parent")(ProcessHandle.of(pid).get().parent().map(h => h.pid() == parent.pid()).orElse(false))
    parent.destroyForcibly()
    show("parent")(parent.waitFor())
    val grandchild = ProcessHandle.of(pid)
    show("grandchild")(grandchild.isPresent && grandchild.get().isAlive())
    show("grandchild ended")(grandchild.get().destroyForcibly())
    // Handles dropped: the children are reaped as they end.
    for i <- 1 to 8 do new ProcessBuilder("true").start()
    Thread.sleep(500)
    show("dropped children")(ProcessHandle.current().children().count())
    // The program itself.
    val me = ProcessHandle.current()
    show("current")(me.pid() > 0 && me.isAlive())
    show("current listed")(ProcessHandle.allProcesses().anyMatch(h => h.pid() == me.pid()))
    show("destroy current")(me.destroy())
    show("of an absent pid")(ProcessHandle.of(Int.MaxValue.toLong - 1))
    show("equal handles")(ProcessHandle.of(me.pid()).get() == me)
    show("missing program")(new ProcessBuilder("no-such-program-of-teq").start())
    show("missing absolute")(new ProcessBuilder("/no/such/program").start())
    show("empty command")(new ProcessBuilder().start())
    show("status 3")(new ProcessBuilder("sh", "-c", "exit 3").start().waitFor())
