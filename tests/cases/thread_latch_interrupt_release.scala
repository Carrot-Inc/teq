//> using platform jvm
// A latch's waiter interrupted, then the latch released, before it runs again: the interrupt ends its wait, as
// JDK 24's acquireSharedInterruptibly checks it as the park returns, and clears it.
import java.util.concurrent.*
object Main:
  def main(args: Array[String]): Unit =
    val latch = new CountDownLatch(1)
    val t = new Thread(() => {
      try
        latch.await()
        println("returned")
      catch case _: InterruptedException => println("interrupted")
      println("flag=" + Thread.currentThread().isInterrupted())
    })
    t.start()
    val end = System.nanoTime() + 2000000000L
    while t.getState() != Thread.State.WAITING && System.nanoTime() < end do Thread.`yield`()
    t.interrupt()
    latch.countDown()
    t.join()
