// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 9:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 1 error found
// A call under a `return` in a `synchronized` body is no loop's jump on the JVM: scalac 3.8.4 rewrites it
// and throws `IllegalMonitorStateException`, teq refuses it.
import scala.annotation.tailrec
object Guarded:
  @tailrec final def throughMonitor(n: Int): Int = this.synchronized { if n > 0 then return throughMonitor(n - 1); 0 }
