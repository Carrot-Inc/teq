// A subclass of a class that mixes in two traits declaring a default: the subclass declares the method without the
// default, so its getter is the parent's own, which it inherits; a forwarder to a trait's getter would replace it.
object ParentGetter:
  trait A { def m(a: Int = 1): Int = a }
  trait B extends A { override def m(a: Int = 2): Int = a + 10 }
  trait C extends A { override def m(a: Int = 3): Int = a + 20 }
  class P extends B with C:
    override def m(a: Int = 9): Int = a + 100
  class K extends P:
    override def m(a: Int): Int = a + 100

  def run(): Unit =
    println(new P().m())
    println(new K().m())
    println((new K(): A).m())

object Main:
  def main(args: Array[String]): Unit = ParentGetter.run()
