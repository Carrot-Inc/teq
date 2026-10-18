//> using platform jvm
// awaitNanos(0) on a fair lock with a thread queued for it: the lock is released whole and taken back
// behind that thread, which runs first, as JDK 24's ConditionObject.awaitNanos does (enableWait, reacquire).
import java.util.concurrent.locks.ReentrantLock
object Main {
  def main(args: Array[String]): Unit = {
    val lock = new ReentrantLock(true)
    val c = lock.newCondition()
    val n = new java.util.concurrent.atomic.AtomicInteger()
    lock.lock()
    val t = new Thread(() => { lock.lock(); try n.set(1) finally lock.unlock() })
    t.start()
    val end = System.nanoTime() + 3000000000L
    while (!lock.hasQueuedThreads() && System.nanoTime() < end) Thread.`yield`()
    c.awaitNanos(0L)
    println(n.get())
    lock.unlock()
    t.join()
  }
}
