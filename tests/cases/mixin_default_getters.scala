// A class mixing in traits takes a forwarder of a default getter where two traits, neither extending the other,
// declare it (`MixinOps.needsForwarder` judges the getter by its own name): the diamond, three traits
// whose linearisation makes the middle one win, a joining trait that drops the default or overrides nothing,
// a class between, the class's own override without the default, a later clause's default, the traits at the
// top level and in a block.
trait TopA { def m(a: Int = 10): Int = a }
trait TopB extends TopA { override def m(a: Int = 11): Int = a + 100 }
trait TopD extends TopA { override def m(a: Int = 12): Int = a + 200 }
class TopC extends TopB with TopD { def inside: Int = m() }

object Diamond:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int = 10 + 2): Int = a + 200 }
  class C extends B with D
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object Three:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int = 10 + 2): Int = a + 200 }
  trait E extends B { override def m(a: Int = 10 + 3): Int = a + 300 }
  class C extends E with D with B
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object Joined:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int = 10 + 2): Int = a + 200 }
  trait E extends B with D { override def m(a: Int): Int = a + 300 }
  class C extends E
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object JoinedPlain:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int = 10 + 2): Int = a + 200 }
  trait E extends B with D
  class C extends E
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object Between:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int = 10 + 2): Int = a + 200 }
  abstract class P extends B
  class C extends P with D
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object OwnOverride:
  trait A { def m(a: Int = 10): Int = a }
  trait B extends A { override def m(a: Int = 10 + 1): Int = a + 100 }
  trait D extends A { override def m(a: Int = 10 + 2): Int = a + 200 }
  class C extends B with D { override def m(a: Int): Int = a + 500 }
  def run(): Unit =
    println(new C().m())
    println((new C(): A).m())

object Curried:
  trait A { def m(p: Int)(a: Int = p): Int = a }
  trait B extends A { override def m(p: Int)(a: Int = p + 1): Int = a + 100 }
  trait D extends A { override def m(p: Int)(a: Int = p + 2): Int = a + 200 }
  class C extends B with D
  def run(): Unit =
    println(new C().m(5)())
    println((new C(): A).m(5)())

object Main:
  def inBlock(): Unit =
    trait A { def m(a: Int = 10): Int = a }
    trait B extends A { override def m(a: Int = 11): Int = a + 100 }
    trait D extends A { override def m(a: Int = 12): Int = a + 200 }
    class C extends B with D
    println(new C().m())
    println((new C(): A).m())

  def main(args: Array[String]): Unit =
    Diamond.run(); Three.run(); Joined.run(); JoinedPlain.run(); Between.run(); OwnOverride.run(); Curried.run()
    println(new TopC().m())
    println((new TopC(): TopA).m())
    println(new TopC().inside)
    inBlock()
