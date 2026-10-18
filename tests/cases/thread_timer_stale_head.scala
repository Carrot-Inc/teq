//> using platform jvm
// A timer a wake made stale at the heap's head hides none behind it: main's 100 ms park is unparked early,
// the worker sleeps 200 ms more, and main, polling with nothing else runnable, sees the worker's flag set.
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.locks.LockSupport
object Main:
  def main(args: Array[String]): Unit =
    val done = new AtomicBoolean(false)
    val main = Thread.currentThread()
    val worker = new Thread(() => {
      Thread.sleep(20)
      LockSupport.unpark(main)
      Thread.sleep(200)
      done.set(true)
    })
    worker.start()
    LockSupport.parkNanos(100000000L)
    val end = System.nanoTime() + 2000000000L
    while !done.get() && System.nanoTime() < end do ()
    println(done.get())
    worker.join()
