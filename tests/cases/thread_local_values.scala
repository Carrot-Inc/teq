//> using platform jvm
// `ThreadLocal` per thread: an initial value computed in each thread that reads it, a value set in
// main unseen by another thread, `remove` back to the initial value; an `InheritableThreadLocal`'s
// value copied into a thread at its creation.
object Main:
  def main(args: Array[String]): Unit =
    val named = ThreadLocal.withInitial[String](() => Thread.currentThread().getName)
    val box = new ThreadLocal[String]
    val inherited = new InheritableThreadLocal[String]
    box.set("main's")
    inherited.set("from main")
    val t = new Thread(() =>
      println("t: " + named.get + ", " + box.get + ", " + inherited.get)
      box.set("t's")
      println("t after set: " + box.get), "other")
    t.start()
    t.join()
    println("main: " + named.get + ", " + box.get + ", " + inherited.get)
    box.remove()
    println("after remove: " + box.get)
