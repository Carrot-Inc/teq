//> using platform jvm
// (JavaScript left out: teq's JavaScript output does not leave the method from a local lazy initialiser's return.)
// One class holding the keys of two lazy vals of one name (the one a local def reads, the one read directly, each
// returning from its own method's invocation) and the instances of two local classes of one name, each the home of a
// local def the class reads: each in a field of its own (LambdaLift's proxies).
// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
import java.util.concurrent.Callable
class Outer(val n: Int):
  def keys(): Int =
    lazy val x: Int = if n > 0 then return n + 1 else 0
    def outerX(): Int = x
    def inner(): Int =
      lazy val x: Int = if n > 0 then return n + 2 else 0
      new Callable[Int] { def call(): Int = outerX() + x }.call()
    inner() * 1000
  def homes(): Int =
    class L(val a: Int):
      def m(): Int =
        def g1(): Int = a + n
        class L(val b: Int):
          def m2(): Int =
            def g2(): Int = b + a
            new Callable[Int] { def call(): Int = g1() * 100 + g2() }.call()
        new L(2).m2()
    new L(1).m()
object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer(40)
    println(List(o.keys(), o.homes()))
