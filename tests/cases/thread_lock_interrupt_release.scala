//> using platform jvm
// An interruptible acquisition interrupted, then the lock released, before the waiter runs again: the
// interrupt wins, as JDK 24's AbstractQueuedSynchronizer.acquire checks it as the park returns, and
// cancels the acquisition.
import java.util.concurrent.locks.ReentrantLock
object Main {
  def main(args: Array[String]): Unit = {
    val lock = new ReentrantLock()
    lock.lock()
    val t = new Thread(() => {
      try {
        lock.lockInterruptibly()
        try println("acquired") finally lock.unlock()
      } catch { case _: InterruptedException => println("interrupted") }
    })
    t.start()
    val end = System.nanoTime() + 3000000000L
    while ((!lock.hasQueuedThreads() || t.getState() != Thread.State.WAITING) && System.nanoTime() < end) Thread.`yield`()
    t.interrupt()
    lock.unlock()
    t.join()
  }
}
