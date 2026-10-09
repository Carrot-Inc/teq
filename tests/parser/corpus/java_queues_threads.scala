// The JDK's queues, thread and runtime as zio's JavaScript bodies call them: `ArrayDeque` at both
// ends, `ConcurrentLinkedQueue` built from a collection, `Thread.currentThread`, `interrupted`,
// `Runtime.availableProcessors`, `ThreadLocalRandom` and `RejectedExecutionException`.
import java.util.ArrayDeque
import java.util.concurrent.{ConcurrentLinkedQueue, RejectedExecutionException, ThreadLocalRandom}

object Main:
  def main(args: Array[String]): Unit =
    val d = new ArrayDeque[String]()
    d.addLast("b")
    d.addFirst("a")
    d.offerLast("c")
    d.push("z")
    println(d)
    println(s"${d.peekFirst()} ${d.peekLast()} ${d.size()}")
    println(s"${d.pollFirst()} ${d.pollLast()} ${d.pop()}")
    println(s"${d.getFirst()} ${d.getLast()} ${d.contains("b")}")
    d.addAll(java.util.List.of("x", "y", "x"))
    println(d.removeLastOccurrence("x"))
    println(d)
    val desc = d.descendingIterator()
    val sb = new StringBuilder
    while desc.hasNext do sb.append(desc.next())
    println(sb)
    d.clear()
    println(s"${d.isEmpty()} ${d.pollFirst()} ${d.peek()}")
    println(scala.util.Try(d.removeFirst()).isFailure)

    val src = new java.util.ArrayList[Int]()
    src.add(3)
    src.add(1)
    val q = new ConcurrentLinkedQueue[Int](src)
    q.offer(2)
    q.add(5)
    println(q)
    println(s"${q.peek()} ${q.poll()} ${q.size()} ${q.isEmpty()}")
    println(q.remove(Integer.valueOf(2)))
    val it = q.iterator()
    var sum = 0
    while it.hasNext do sum += it.next()
    println(sum)
    println(q.toArray().length)
    q.clear()
    println(q.isEmpty())

    val t = Thread.currentThread()
    println(t.getName())
    println(Thread.interrupted())
    println(Runtime.getRuntime().availableProcessors() > 0)
    val r = ThreadLocalRandom.current().nextInt(10)
    println(r >= 0 && r < 10)
    val e = new RejectedExecutionException("Unable to run task")
    println(e.getMessage)
    println(e.isInstanceOf[RuntimeException])
