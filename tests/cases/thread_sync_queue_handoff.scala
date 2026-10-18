//> using platform jvm
// A SynchronousQueue's offer is matched to the waiting taker before it succeeds, as JDK 24's
// `Transferer.xferLifo` has it: a poll right after finds nothing, and the taker has the element.
import java.util.concurrent.SynchronousQueue
import java.util.concurrent.atomic.AtomicReference
object Test {
 def main(args: Array[String]): Unit = {
  val q = new SynchronousQueue[String]()
  val received = new AtomicReference[String]("pending")
  val t = new Thread(() => { try received.set(q.take()) catch { case _: InterruptedException => received.set("interrupted") } })
  t.start()
  val until = System.nanoTime() + 2000000000L
  while t.getState() != Thread.State.WAITING && System.nanoTime() < until do Thread.`yield`()
  val offered = q.offer("task")
  val stolen = q.poll()
  t.join(200)
  t.interrupt(); t.join()
  println(offered); println(stolen); println(received.get())
 }
}
