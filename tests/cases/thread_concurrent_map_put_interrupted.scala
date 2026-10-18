//> using platform jvm
// A ConcurrentHashMap put by a thread whose interrupt is set, while another thread's compute holds the key:
// the put waits for the remapping and keeps the interrupt, as JDK 24's putVal blocks on the bin's lock, which
// an interrupt does not end.
import java.util.concurrent.*
object Main:
  def main(args: Array[String]): Unit =
    val map = new ConcurrentHashMap[String, String]()
    map.put("key", "old")
    val entered = new CountDownLatch(1)
    val finish = new CountDownLatch(1)
    val remapper = new Thread(() => {
      map.compute("key", (_, v) => {
        entered.countDown()
        finish.await()
        v + "+computed"
      })
      ()
    })
    remapper.start()
    entered.await()
    val writer = new Thread(() => {
      Thread.currentThread().interrupt()
      val before = map.put("key", "put")
      println("put saw=" + before + " interrupted=" + Thread.interrupted())
    })
    writer.start()
    while writer.getState() == Thread.State.RUNNABLE || writer.getState() == Thread.State.NEW do Thread.sleep(1)
    println("writer waiting=" + writer.isAlive())
    finish.countDown()
    remapper.join()
    writer.join()
    println("value=" + map.get("key"))
