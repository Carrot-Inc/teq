//> using platform jvm
// An exception a thread does not catch: the JDK prints its `Exception in thread "<name>"` report on
// stderr and the thread ends, main unaffected; a thread's own handler, then the default handler,
// receives it in place of the report.
object Main:
  def fail(message: String): Unit = throw new IllegalStateException(message)

  def main(args: Array[String]): Unit =
    val t = new Thread(() => fail("boom"), "worker")
    t.start()
    t.join()
    println("main continues, worker alive " + t.isAlive)
    val h = new Thread(() => fail("handled"), "own")
    h.setUncaughtExceptionHandler((th, e) => println("handler: " + th.getName + " " + e.getMessage))
    h.start()
    h.join()
    Thread.setDefaultUncaughtExceptionHandler((th, e) => println("default: " + th.getName + " " + e.getMessage))
    val d = new Thread(() => fail("by default"), "plain")
    d.start()
    d.join()
    println("main ends")
