//> using platform jvm
// A lazy val read by a second thread while the first evaluates it: the second waits for the value,
// as dotty's `LazyVals.mkThreadSafeDef` has it, and the body runs once.
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicInteger
class Box(entered: CountDownLatch) {
 val calls = new AtomicInteger()
 lazy val value: Int = { calls.incrementAndGet(); entered.countDown(); Thread.sleep(200); 42 }
}
object Test {
 def main(args: Array[String]): Unit = {
  val ready = new CountDownLatch(1)
  val b = new Box(ready)
  val t = new Thread(() => { println(b.value) })
  t.start(); ready.await(); println(b.value); t.join(); println(b.calls.get())
 }
}
