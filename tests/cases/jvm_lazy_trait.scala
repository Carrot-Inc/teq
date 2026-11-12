//> using platform jvm
// A trait's lazy val held by a class and by an object, read by sixteen threads released together, is computed once in
// each (the class holding it writes the thread-safe member, LazyVals.transformLazyVal leaving the trait's to it).
import java.util.concurrent.{CountDownLatch, Executors}
import java.util.concurrent.atomic.AtomicInteger

trait Pooled:
  val count = new AtomicInteger(0)
  lazy val pool: String = { count.incrementAndGet(); Thread.sleep(50); "pool" }

class Service extends Pooled
object Shared extends Pooled

@main def run(): Unit =
  val n = 16
  val wrong = new AtomicInteger(0)
  val service = new Service
  val ex = Executors.newFixedThreadPool(n)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(n)
  for _ <- 1 to n do
    ex.submit(new Runnable {
      def run(): Unit =
        try { start.await(); if service.pool != "pool" || Shared.pool != "pool" then wrong.incrementAndGet() }
        catch { case _: Throwable => wrong.incrementAndGet() }
        finally done.countDown()
    })
  start.countDown(); done.await(); ex.shutdown()
  println(s"a trait's lazy val in a class: initialised ${service.count.get} time(s), in an object: ${Shared.count.get} time(s), ${wrong.get} wrong read(s)")
