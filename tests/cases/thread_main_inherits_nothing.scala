//> using platform jvm
// The main thread, made at the first Thread.currentThread(), takes no InheritableThreadLocal value, as the JDK
// attaches it without inheritance: childValue runs for the child alone.
object Main:
  def main(args: Array[String]): Unit =
    val local = new InheritableThreadLocal[String]():
      override def childValue(parent: String): String =
        println("copy=" + parent)
        parent
    local.set("parent")
    println("name=" + Thread.currentThread().getName())
    val t = new Thread(() => println("child=" + local.get()))
    t.start()
    t.join()
