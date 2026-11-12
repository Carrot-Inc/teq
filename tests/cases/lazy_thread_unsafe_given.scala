//> using platform jvm
// A `@threadUnsafe` given alias is a lazy val to dotty (Parsers.givenDef), so LazyVals.transformLazyVal writes it
// unsynchronised: two threads are inside its initialiser at once, each printing the value. A thread-safe one keeps
// the second reader waiting for the first, which here waits (three seconds at most) for the second to enter, and
// the last line says so.
import java.util.concurrent.{CountDownLatch, TimeUnit}
class Holder:
  val entered = new CountDownLatch(2)
  @scala.annotation.threadUnsafe given x: Int = {
    entered.countDown()
    entered.await(3, TimeUnit.SECONDS)
    42
  }
@main def run(): Unit =
  val h = new Holder
  val a = new Thread(() => println(h.x))
  val b = new Thread(() => println(h.x))
  a.start(); b.start(); a.join(); b.join()
  println(s"both threads were inside the initialiser: ${h.entered.getCount == 0}")
