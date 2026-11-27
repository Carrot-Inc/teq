// A class takes no forwarder of a member its superclass's traits declare: only the traits it mixes in itself are
// its mixins, so a subclass of a class over two traits inherits the class's own member (or its forwarder).
object ThroughSuperclass:
  trait A { def m(a: Int = 1): Int = a }
  trait B extends A { override def m(a: Int = 2): Int = a + 10 }
  trait C extends A { override def m(a: Int = 3): Int = a + 20 }
  class P extends B with C:
    override def m(a: Int = 9): Int = a + 100
  class K extends P:
    override def m(a: Int): Int = a + 100
  class K2 extends P
  class Q extends B with C
  class K3 extends Q

  def run(): Unit =
    println(s"${new P().m()} ${new K().m()} ${(new K(): A).m()}")
    println(s"${new K2().m()} ${new K2().m(5)} ${(new K2(): A).m(5)} ${(new K2(): A).m()}")
    println(s"${new Q().m()} ${new K3().m()} ${(new K3(): A).m()}")

object Main:
  def main(args: Array[String]): Unit = ThroughSuperclass.run()
