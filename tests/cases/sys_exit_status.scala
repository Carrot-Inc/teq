//> using platform jvm
// `sys.exit` ends the program with its status, as the JVM's process ends: nothing after the call
// runs, a `finally` and a `catch` of `Throwable` included, and what was printed before it is out.
// Run as every case is, without arguments, it prints and returns; tests/run_interp.sh runs it
// again with one, where it exits with status 2 after its line on stderr, as scalac's run does.
object Main:
  def stop(status: Int): Unit =
    try
      try sys.exit(status)
      catch case t: Throwable => println("caught " + t)
    finally println("finally")

  def main(args: Array[String]): Unit =
    println("before the exit")
    if args.nonEmpty then
      System.err.println("usage: sys_exit_status")
      stop(2)
      println("after the exit")
    else println("no exit without an argument")
