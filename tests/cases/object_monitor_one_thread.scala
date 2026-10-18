// Object.wait, notify and notifyAll on one thread that holds the object's monitor: the signals reach no waiter,
// a timed wait returns at its time, and each receiver is evaluated before the arguments, on every target.
object Main:
  def main(args: Array[String]): Unit =
    val o = new Object
    def recv(tag: String): Object =
      println("receiver " + tag)
      o
    o.synchronized {
      recv("notify").notify()
      recv("notifyAll").notifyAll()
      recv("wait").wait({ println("argument"); 1L })
      recv("wait2").wait(1L, { println("nanos"); 5 })
    }
    println("done")
