//> using platform jvm
// The main thread spins on an `AtomicBoolean` without ever blocking while a thread it started sleeps
// and then sets it: the spinning thread has to let the sleeper's timer and the sleeper run, as the
// JDK's threads run beside each other.
import java.util.concurrent.atomic.AtomicBoolean

object Main:
  def main(args: Array[String]): Unit =
    val ready = new AtomicBoolean(false)
    val worker = new Thread(() => { Thread.sleep(20); ready.set(true) })
    worker.start()
    val until = System.nanoTime() + 5000000000L
    var spins = 0L
    while !ready.get() && System.nanoTime() < until do spins += 1
    println(ready.get())
    worker.join()
    println(spins > 0)
