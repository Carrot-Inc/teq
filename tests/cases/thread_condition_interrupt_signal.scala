//> using platform jvm
// A signal and an interrupt of the same condition waiter, as JDK 24's ConditionObject arbitrates them: the first
// waiter is interrupted, then the condition signalled, before it runs again; one of the two waits ends signalled.
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.locks.ReentrantLock
object Main:
  def main(args: Array[String]): Unit =
    val lock = new ReentrantLock()
    val cond = lock.newCondition()
    val ready = new CountDownLatch(2)
    val signalled = new AtomicInteger(0)
    def waiter(): Thread = new Thread(() => {
      lock.lock()
      try
        ready.countDown()
        try
          if cond.await(2, TimeUnit.SECONDS) then signalled.incrementAndGet()
        catch case _: InterruptedException => ()
      finally lock.unlock()
    })
    val a = waiter()
    a.start()
    Thread.sleep(50)
    val b = waiter()
    b.start()
    ready.await()
    lock.lock()
    try
      a.interrupt()
      cond.signal()
    finally lock.unlock()
    a.join()
    b.join()
    println(signalled.get())
