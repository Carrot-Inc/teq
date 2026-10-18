//> using platform jvm
// A local lazy val read by a second thread while the first evaluates it: the second waits for the value,
// as dotty's `LazyVals.transformLocalDef` runs the initialiser synchronized on its holder, and the body runs once.
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicInteger
object Test {
  def main(args: Array[String]): Unit = {
    val calls = new AtomicInteger()
    val entered = new CountDownLatch(1)
    lazy val value: Int = { calls.incrementAndGet(); entered.countDown(); Thread.sleep(200); 42 }
    val t = new Thread(() => println(value))
    t.start(); entered.await(); println(value); t.join(); println(calls.get())
  }
}
