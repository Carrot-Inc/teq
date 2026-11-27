// An override that drops its default, beside an overload: a missing argument applies where the overridden member
// declares the default (dotty's hasDefaultParams over the overridden members, Applications.findDefaultGetter's
// testOnly lookup), a trait's member and a class's alike; the default and the body that run are the override's
// declarer's and the override's. scalac 3.8.4 prints 1002, 110 and 4.
object Lib {
  trait A { def m(a: Int = 100): Int = a * 10 + 1; def m(s: String): Int = s.length }
  trait B extends A { override def m(a: Int): Int = a * 10 + 2 }
  class C extends A with B
  class P { def k(a: Int = 11): Int = a * 10; def k(s: String): Int = s.length }
  class Q extends P { override def k(a: Int): Int = a * 10 }
  class R { def n(a: Int, b: Int = 3): Int = a + b; def n(s: String): Int = s.length }
  class T extends R { override def n(a: Int, b: Int): Int = a * b }
}
@main def run(): Unit =
  println(new Lib.C().m())
  println(new Lib.Q().k())
  println(new Lib.T().n(2) - 2)
