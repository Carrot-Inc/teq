//> using platform jvm
// Interruption as the JDK has it: a sleeping thread interrupted wakes with `InterruptedException`,
// its flag cleared; an interrupt set before a sleep makes the sleep throw at once; `interrupted`
// answers and clears the flag, `isInterrupted` only answers; a thread waiting in `join` is woken too.
object Main:
  def main(args: Array[String]): Unit =
    val sleeper = new Thread(() =>
      try
        Thread.sleep(10000)
        println("not interrupted")
      catch case e: InterruptedException => println("interrupted: " + e.getMessage + ", flag " + Thread.currentThread().isInterrupted))
    sleeper.start()
    Thread.sleep(50)
    sleeper.interrupt()
    sleeper.join()
    Thread.currentThread().interrupt()
    println(Thread.currentThread().isInterrupted)
    try Thread.sleep(1000)
    catch case e: InterruptedException => println("at once: " + e.getMessage)
    println(Thread.interrupted())
    Thread.currentThread().interrupt()
    println(Thread.interrupted())
    println(Thread.interrupted())
    val main = Thread.currentThread()
    val blocker = new Thread(() => { Thread.sleep(50); main.interrupt() })
    val forever = new Thread(() => Thread.sleep(400))
    forever.start()
    blocker.start()
    try forever.join()
    catch case e: InterruptedException => println("join interrupted: " + e.getMessage)
    forever.join()
    blocker.join()
