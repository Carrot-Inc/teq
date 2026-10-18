//> using platform jvm
// A thread's identity and states: main's name, a started thread's own name and number, distinct ids,
// `currentThread` inside it, the priority it keeps, `Thread.yield` and `onSpinWait` accepted, the
// thread's states before, during and after its run, its `toString`.
import java.util.concurrent.CountDownLatch

object Main:
  def main(args: Array[String]): Unit =
    val main = Thread.currentThread()
    println(s"${main.getName} ${main.isDaemon} ${main.getPriority}")
    val inside = new CountDownLatch(1)
    val release = new CountDownLatch(1)
    var seen = ""
    val t = new Thread(() =>
      seen = Thread.currentThread().getName
      inside.countDown()
      release.await())
    t.setPriority(Thread.MAX_PRIORITY)
    println(s"${t.getName.startsWith("Thread-")} ${t.getPriority} ${t.getId != main.getId} ${t.threadId == t.getId}")
    t.setName("renamed")
    t.start()
    inside.await()
    println(seen)
    Thread.`yield`()
    Thread.onSpinWait()
    release.countDown()
    t.join()
    println(s"${t.getState} ${t.isAlive}")
    println(t.toString.startsWith("Thread[#"))
