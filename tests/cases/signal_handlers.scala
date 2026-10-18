//> using platform jvm
// os: unix
// `sun.misc.Signal` as JDK 24 has it: a handler of USR1 and one of INT installed, each signal sent by the process
// to itself and handled on a daemon thread named after it, INT without the shutdown; the handler `handle`
// replaces, the default first; the signals the VM keeps and the names it does not know refused.
import sun.misc.{Signal, SignalHandler}
import java.util.concurrent.{CountDownLatch, TimeUnit}

object Main:
  def kill(sig: String): Unit =
    new ProcessBuilder("kill", "-" + sig, ProcessHandle.current().pid().toString).start().waitFor()

  def main(args: Array[String]): Unit =
    val seen = new CountDownLatch(1)
    val usr1 = new Signal("USR1")
    println(s"$usr1 ${usr1.getName}")
    val handler: SignalHandler = sig =>
      val t = Thread.currentThread()
      println(s"handled $sig on ${t.getName}, daemon ${t.isDaemon}")
      seen.countDown()
    val first = Signal.handle(usr1, handler)
    println("the default replaced: " + (first eq SignalHandler.SIG_DFL))
    kill("USR1")
    println("seen: " + seen.await(5, TimeUnit.SECONDS))
    val second = Signal.handle(usr1, SignalHandler.SIG_IGN)
    println("the handler replaced: " + (second eq handler))
    kill("USR1")
    val int = new CountDownLatch(1)
    Signal.handle(new Signal("INT"), _ => int.countDown())
    kill("INT")
    println("INT handled, no shutdown: " + int.await(5, TimeUnit.SECONDS))
    for name <- List("QUIT", "SEGV", "KILL", "SIGINT", "NOPE") do
      try
        Signal.handle(new Signal(name), handler)
        println(name + ": installed")
      catch case e: IllegalArgumentException => println(name + ": " + e.getMessage)
    println("done")
