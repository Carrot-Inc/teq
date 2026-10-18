//> using platform jvm
// A ConcurrentHashMap read while another thread's compute runs its remapping: the read proceeds and sees the
// value before it, as JDK 24's get takes no lock and compute locks the key's bin alone.
import java.util.concurrent.*
object Main:
  def main(args: Array[String]): Unit =
    val map = new ConcurrentHashMap[String, String]()
    map.put("key", "old")
    val entered = new CountDownLatch(1)
    val read = new CountDownLatch(1)
    val t = new Thread(() => {
      entered.await()
      println("read=" + map.get("key"))
      read.countDown()
    })
    t.start()
    map.compute("key", (_, _) => {
      entered.countDown()
      println("reader-progress=" + read.await(1, TimeUnit.SECONDS))
      "new"
    })
    t.join()
