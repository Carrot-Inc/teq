//> using platform jvm
// An inner class's enclosing instance is checked for null when the instance is made, and what a class captures,
// its enclosing instance included, is set before its superclass's constructor runs, which may read it through an
// overridden member (dotty's `Constructors`: the parameters' fields, then the outer's null check and the outer,
// then the superclass call). The JVM's platform: JavaScript and the interpreter make the instance on a null outer.
abstract class A { def f: String; val y: String = f }
class O(val k: Int) {
  class I(val j: Int) { def show: String = s"$j" }
  class E extends A { def f: String = s"$k" }
  class X(val m: String) extends Exception { override def fillInStackTrace(): Throwable = { O.seen = s"$k $m"; this } }
}
object O { var seen: String = "" }
trait T { class Ti(val j: Int) }
object Main:
  def attempt(body: => Any): String =
    try s"made $body"
    catch case e: NullPointerException => s"npe at ${e.getStackTrace.head.getMethodName}"
  def run(q: Int, s: String): String =
    class L extends A { def f: String = s"$q $s" }
    val a = new A { def f: String = s"$q $s" }
    s"${(new L).y} | ${a.y}"
  def main(args: Array[String]): Unit =
    val o = new O(7)
    val n: O = null
    val t: T = null
    println(attempt((new o.I(1)).show))
    println(attempt((new n.I(1)).show))
    println(attempt((new o.E).y))
    println(attempt((new n.E).y))
    println(attempt(new t.Ti(2).j))
    println(attempt { new o.X("x"); O.seen })
    println(run(7, "s"))
