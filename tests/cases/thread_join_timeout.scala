//> using platform jvm
// A thread that sleeps, joined first with a timeout shorter than its sleep (it is still alive and
// sleeping, `TIMED_WAITING`), then without one (it has ended, `TERMINATED`); a thread's states
// before it starts and a second `start`, which the JDK refuses.
object Main:
  def main(args: Array[String]): Unit =
    val t = new Thread(() => { Thread.sleep(300); println("slept") })
    println(t.getState)
    t.start()
    t.join(30)
    println("after join(30): alive " + t.isAlive + ", " + t.getState)
    t.join()
    println("after join(): alive " + t.isAlive + ", " + t.getState)
    try t.start()
    catch case e: IllegalThreadStateException => println("start again: " + e.getClass.getName)
    t.join(10)
    println("join of an ended thread returns")
