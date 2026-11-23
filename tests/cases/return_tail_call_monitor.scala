//> using platform jvm
// expected: by hand
// A call of a def by itself under a `return` in a `synchronized` body is a call: each frame's monitor is
// exited as it returns. scalac 3.8.4 makes it a loop that enters the monitor again on each round and
// throws `java.lang.IllegalMonitorStateException` when it exits it.
object Monitor {
  final def down(n: Int): Int = this.synchronized { if n > 0 then return down(n - 1); if Thread.holdsLock(this) then 0 else -1 }
}

object Main {
  def main(args: Array[String]): Unit = {
    println(try Monitor.down(3).toString catch { case e: Throwable => e.toString })
    println("held after " + Thread.holdsLock(Monitor))
  }
}
