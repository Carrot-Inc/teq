//> using platform jvm
// A hand-over through an object's monitor: the consumer waits in a loop until main fills the slot
// and notifies, empties it and notifies back; `wait` without the monitor is refused with the JDK's
// `IllegalMonitorStateException`; a timed wait returns when its time is up.
object Main:
  private val lock = new Object
  private var slot: String = null

  def main(args: Array[String]): Unit =
    val consumer = new Thread(() =>
      lock.synchronized {
        while slot == null do lock.wait()
        println("consumer got " + slot)
        slot = null
        lock.notifyAll()
      })
    consumer.start()
    lock.synchronized {
      slot = "the value"
      lock.notifyAll()
      while slot != null do lock.wait()
    }
    consumer.join()
    println("handed over")
    try lock.wait()
    catch case e: IllegalMonitorStateException => println("wait: " + e)
    try lock.notify()
    catch case e: IllegalMonitorStateException => println("notify: " + e)
    lock.synchronized { lock.wait(5) }
    println("timed wait returned")
