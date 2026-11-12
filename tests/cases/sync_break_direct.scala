//> using platform jvm
// split-watch: jvm
// A `boundary.break` straight out of a `synchronized` block in its boundary's method is to release the
// monitor and return the value. The expectation is teq's own, which parts from scalac here on purpose:
// scalac 3.8.4's DropBreaks makes that break a labelled return, which scalac's backend writes as a jump
// past the monitorexit, and the JVM throws IllegalMonitorStateException at the method's return. Through
// a `try`, or from a local def, the break stays an exception and scalac releases the monitor too.
import scala.util.boundary, boundary.break

object Main:
  val lock = new Object
  def direct(): Int = boundary { lock.synchronized { if lock ne null then break(1); 2 } }
  def throughTry(): Int = boundary { try lock.synchronized { if lock ne null then break(3); 4 } finally () }
  def throughDef(): Int = boundary {
    def f(): Int = lock.synchronized { if lock ne null then break(5); 6 }
    f()
  }

  def main(args: Array[String]): Unit =
    println(s"direct ${direct()} held=${Thread.holdsLock(lock)}")
    println(s"through try ${throughTry()} held=${Thread.holdsLock(lock)}")
    println(s"through a local def ${throughDef()} held=${Thread.holdsLock(lock)}")
