//> using platform jvm
// A timed condition wait signalled at once, its lock then held past the time: `await(time, unit)` reports the
// signal, as JDK 24's ConditionObject returns `!cancelled` whatever taking the lock back took.
import java.util.concurrent.locks.ReentrantLock
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
object Test {
 def main(args: Array[String]): Unit = {
  val lock = new ReentrantLock()
  val cond = lock.newCondition()
  val ready = new CountDownLatch(1)
  val t = new Thread(() => { lock.lock(); try { ready.countDown(); println(cond.await(400, TimeUnit.MILLISECONDS)) } finally lock.unlock() })
  t.start(); ready.await(); lock.lock()
  try { cond.signal(); Thread.sleep(700) } finally lock.unlock()
  t.join()
 }
}
