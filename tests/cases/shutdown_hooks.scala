//> using platform jvm
// os: unix
// `Runtime.addShutdownHook` against the JDK: the hook's thread runs when the program ends, after
// main's last line and its `finally`; a hook registered twice is refused, one removed does not run.
// Run as every case is, without arguments, main returns; tests/run_interp.sh runs it again with
// `exit` (System.exit(3): the hook runs, the `finally` does not, the status is 3), `term` (the
// program sends itself SIGTERM while it sleeps: the hook runs, the `finally` does not, the status is
// 143) and `throw` (an uncaught exception: the hook runs after it is reported, the status is 1).
object Main:
  def main(args: Array[String]): Unit =
    val hook = new Thread(() => println("the hook runs"))
    Runtime.getRuntime.addShutdownHook(hook)
    try Runtime.getRuntime.addShutdownHook(hook)
    catch case e: IllegalArgumentException => println("twice: " + e.getMessage)
    val removed = new Thread(() => println("the removed hook runs"))
    Runtime.getRuntime.addShutdownHook(removed)
    println("removed: " + Runtime.getRuntime.removeShutdownHook(removed))
    println("removed again: " + Runtime.getRuntime.removeShutdownHook(removed))
    try
      println("main runs")
      args.headOption match
        case Some("exit") => System.exit(3)
        case Some("term") =>
          new ProcessBuilder("kill", "-TERM", ProcessHandle.current().pid().toString).start().waitFor()
          Thread.sleep(20000)
        case Some("throw") => throw new IllegalStateException("thrown out of main")
        case _ => ()
      println("main ends")
    finally println("finally")
