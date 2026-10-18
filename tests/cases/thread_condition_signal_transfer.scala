//> using platform jvm
// A signal moves its waiter to the lock's queue at once, as JDK 24's ConditionObject.doSignal enqueues it:
// the signaller sees it queued, and on a fair lock its release and immediate relock let the waiter run first.
import java.util.concurrent.CountDownLatch
import java.util.concurrent.locks.ReentrantLock
object Main {
  def main(args: Array[String]): Unit = {
    val lock = new ReentrantLock(true)
    val cond = lock.newCondition()
    val ready = new CountDownLatch(1)
    var done = false
    val t = new Thread(() => {
      lock.lock()
      try { ready.countDown(); cond.await(); done = true }
      finally lock.unlock()
    })
    t.start()
    ready.await()
    lock.lock()
    cond.signal()
    println("queued=" + lock.hasQueuedThreads())
    lock.unlock()
    lock.lock()
    try println("done=" + done) finally lock.unlock()
    t.join()
  }
}
