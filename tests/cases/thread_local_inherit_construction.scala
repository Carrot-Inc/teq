//> using platform jvm
// An InheritableThreadLocal set again between a thread's construction and its start: the thread has the value
// it had at its construction, as JDK 24's Thread constructor takes it (ThreadLocal.createInheritedMap).
object Test {
 def main(args: Array[String]): Unit = {
  val local = new InheritableThreadLocal[String]()
  local.set("constructed")
  val t = new Thread(() => println(local.get()))
  local.set("started")
  t.start(); t.join()
 }
}
