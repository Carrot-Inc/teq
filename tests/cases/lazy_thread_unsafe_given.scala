//> using platform jvm
// A `@threadUnsafe` given alias is a lazy val to dotty (Parsers.givenDef), so LazyVals.transformLazyVal writes it
// unsynchronised: two threads both inside its initialiser at once, each printing the value. A thread-safe one would
// keep the second reader waiting for the first, which waits for the second to enter.
import java.util.concurrent.CountDownLatch
class Holder:
  val entered = new CountDownLatch(2)
  @scala.annotation.threadUnsafe given x: Int = {
    entered.countDown()
    entered.await()
    42
  }
@main def run(): Unit =
  val h = new Holder
  val a = new Thread(() => println(h.x))
  val b = new Thread(() => println(h.x))
  a.start(); b.start(); a.join(); b.join()
