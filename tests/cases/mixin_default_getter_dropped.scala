// The diamond where one trait changes the default and the other drops it: the class's getter is the changer's,
// which extends the root's, so the JVM finds it alone and the class takes no forwarder of it, only of the method.
object Dropped:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int): Int = a + 200 }
  class C extends B with D
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object Main:
  def main(args: Array[String]): Unit = Dropped.run()
