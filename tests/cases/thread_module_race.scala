//> using platform jvm
// An object read by a second thread while the first initialises it: the second waits until the
// initialisation completes, as the JVM's class initialisation has it.
import java.util.concurrent.CountDownLatch
object Gate { val entered = new CountDownLatch(1) }
object Slow { Gate.entered.countDown(); Thread.sleep(200); val value = "ready" }
object Test {
 def main(args: Array[String]): Unit = {
  val t = new Thread(() => { println(Slow.value) })
  t.start(); Gate.entered.await(); println(Slow.value); t.join()
 }
}
