//> using platform jvm
// An object's lazy val and an instance's read by sixteen threads released together are each computed once (dotty's
// LazyVals.transformMemberDefThreadSafe): a lazy val that holds a pool must not make one per thread.
import java.util.concurrent.{CountDownLatch, Executors}
import java.util.concurrent.atomic.AtomicInteger

object Holder:
  val count = new AtomicInteger(0)
  lazy val pool: String = { count.incrementAndGet(); Thread.sleep(50); "pool" }

class Inst:
  val count = new AtomicInteger(0)
  lazy val v: String = { count.incrementAndGet(); Thread.sleep(50); "v" }

@main def run(): Unit =
  val n = 16
  val ex = Executors.newFixedThreadPool(n)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(n)
  val inst = new Inst
  for _ <- 1 to n do
    ex.submit(new Runnable { def run(): Unit = { start.await(); Holder.pool; inst.v; done.countDown() } })
  start.countDown(); done.await(); ex.shutdown()
  println(s"object lazy val initialised ${Holder.count.get} time(s), instance lazy val ${inst.count.get} time(s)")
