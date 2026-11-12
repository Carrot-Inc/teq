//> using platform jvm
// A local lazy val read through a closure by sixteen threads released together is computed once (dotty's
// LazyVals.transformLocalDef: a LazyRef holder, its initialiser under the holder's monitor).
import java.util.concurrent.{CountDownLatch, Executors}
import java.util.concurrent.atomic.AtomicInteger

@main def run(): Unit =
  val n = 16
  val count = new AtomicInteger(0)
  val wrong = new AtomicInteger(0)
  lazy val local: String = { count.incrementAndGet(); Thread.sleep(50); "local" }
  val read: () => String = () => local
  val ex = Executors.newFixedThreadPool(n)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(n)
  for _ <- 1 to n do
    ex.submit(new Runnable {
      def run(): Unit =
        try { start.await(); if read() != "local" then wrong.incrementAndGet() }
        catch { case _: Throwable => wrong.incrementAndGet() }
        finally done.countDown()
    })
  start.countDown(); done.await(); ex.shutdown()
  println(s"local lazy val read through a closure: initialised ${count.get} time(s), ${wrong.get} wrong read(s)")
