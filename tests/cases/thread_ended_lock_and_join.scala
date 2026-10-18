//> using platform jvm
// A thread that ends holding a ReentrantLock keeps it, as the JDK's Sync keeps its owner until an unlock;
// joining a thread that has ended returns at once, the joiner's interrupt left set (JDK 24's join checks
// isAlive before it waits).
import java.util.concurrent.locks.ReentrantLock
object Main:
  def main(args: Array[String]): Unit =
    val lock = new ReentrantLock()
    val t = new Thread(() => lock.lock())
    t.start()
    t.join()
    println("orphan-lock=" + lock.tryLock())
    Thread.currentThread().interrupt()
    try
      t.join()
      println("joined-completed")
    catch case _: InterruptedException => println("join-interrupted")
    println("flag=" + Thread.interrupted())
