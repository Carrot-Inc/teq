//> using platform jvm
// `synchronized` as a monitor: eight threads each add to a counter two thousand times, reading it,
// yielding and writing it back inside the block, so that a thread is suspended while it holds the
// monitor; the count is exact, the monitor reentrant, `Thread.holdsLock` true inside it only.
object Main:
  private var count = 0
  private val lock = new Object

  def main(args: Array[String]): Unit =
    val threads = (1 to 8).map(_ => new Thread(() =>
      for _ <- 1 to 2000 do
        lock.synchronized {
          val c = count
          Thread.`yield`()
          count = c + 1
        }))
    threads.foreach(_.start())
    threads.foreach(_.join())
    println(count)
    lock.synchronized { lock.synchronized { println("reentrant") } }
    println(Thread.holdsLock(lock))
    lock.synchronized { println(Thread.holdsLock(lock)) }
