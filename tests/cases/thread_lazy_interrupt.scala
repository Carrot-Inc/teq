//> using platform jvm
// A reader waiting for another thread's lazy val, interrupted, then the value published before it runs: the
// interrupt ends its wait, as the Waiting latch's await checks it as the park returns (dotty's LazyVals).
object Main {
  class Holder {
    var worker: Thread = null
    var result = "pending"
    lazy val x: Int = {
      worker = new Thread(() => {
        try { val v = x; result = "value=" + v }
        catch { case _: InterruptedException => result = "interrupted" }
      })
      worker.start()
      val end = System.nanoTime() + 2000000000L
      while worker.getState() != Thread.State.WAITING && System.nanoTime() < end do Thread.`yield`()
      println("waiting=" + (worker.getState() == Thread.State.WAITING))
      worker.interrupt()
      42
    }
  }
  def main(args: Array[String]): Unit = {
    val h = new Holder
    println("main=" + h.x)
    h.worker.join()
    println(h.result)
  }
}
