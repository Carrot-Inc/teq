//> using platform jvm
// A capacity-one `LinkedBlockingQueue` between two threads, as the JDK has it: the producer's second
// `put` blocks until the consumer's `take` makes room, a timed `offer` gives up on a full queue and a
// timed `poll` on an empty one.
import java.util.concurrent.{LinkedBlockingQueue, TimeUnit}

object Main:
  def main(args: Array[String]): Unit =
    val q = new LinkedBlockingQueue[String](1)
    val producer = new Thread(() =>
      q.put("delivered")
      q.put("second"))
    producer.start()
    println(q.take())
    println(q.take())
    producer.join()
    println(q.offer("a", 10, TimeUnit.MILLISECONDS))
    println(q.offer("b", 10, TimeUnit.MILLISECONDS))
    println(q.remainingCapacity())
    println(q.poll(10, TimeUnit.MILLISECONDS))
    println(q.poll(10, TimeUnit.MILLISECONDS))
