//> using platform jvm
// wait and notify on a null receiver: NullPointerException, as the JVM's invokevirtual throws once the
// arguments are evaluated, before the times are checked.
object Main {
  def main(args: Array[String]): Unit = {
    val x: Object = null
    try x.wait(1L) catch { case e: Exception => println(e.getClass.getSimpleName) }
    try x.notify() catch { case e: Exception => println(e.getClass.getSimpleName) }
  }
}
