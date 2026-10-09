// java.util.concurrent.atomic on one thread: the cells with the JDK's names.
import java.util.concurrent.atomic.{AtomicBoolean, AtomicInteger, AtomicLong, AtomicReference}

object Main:
  def main(args: Array[String]): Unit =
    val i = new AtomicInteger(5)
    println(i.get() + " " + i.incrementAndGet() + " " + i.getAndIncrement() + " " + i.get())
    println(i.compareAndSet(7, 10).toString + " " + i.compareAndSet(7, 11) + " " + i)
    println(i.addAndGet(-4) + " " + i.getAndAdd(2) + " " + i.decrementAndGet())
    i.set(3)
    println(i.intValue + " " + i.longValue + " " + (i.doubleValue / 2) + " " + (i.floatValue / 4) + " " + i.getAndSet(9) + " " + i.get())
    println(i.updateAndGet(x => x * 2) + " " + i.getAndUpdate(x => x + 1) + " " + i.get())
    val l = new AtomicLong(1L << 40)
    println(l.incrementAndGet().toString + " " + l.compareAndSet(1L << 40, 0L) + " " + l.get())
    val b = new AtomicBoolean()
    println(b.get().toString + " " + b.compareAndSet(false, true) + " " + b.getAndSet(false) + " " + b.get())
    val r = new AtomicReference[String]("a")
    println(r.get() + " " + r.compareAndSet("a", "b") + " " + r.compareAndSet("a", "c") + " " + r.get())
    println(r.updateAndGet(_ + "!") + " " + r.getAndSet("z") + " " + r)
