// A super call whose argument holds a branch (a conditional, a match, a try with a handler, a throw, another super call
// with one): the superclass's method, the one a named parent gives and a trait's, each called from a class's method,
// an object's and a lambda in a method. On the JVM the call's receiver is the instance itself wherever the arguments
// join or leave, as the verifier takes it.
class Base { def g(a: Int, b: Int): Int = a * 1000 + b }
trait T { def h(a: Int, b: Int): Int = a * 100 + b }

class C extends Base with T {
  def ifs(n: Int): String = s"${super.g(1, if n > 0 then 2 else 3)} ${super[Base].g(1, if n > 0 then 2 else 3)} ${super[T].h(1, if n > 0 then 2 else 3)}"
  def matches(n: Int): String = s"${super.g(1, n match { case 0 => 10; case _ => 20 })} ${super[T].h(1, n match { case 0 => 10; case _ => 20 })}"
  def tries(n: Int): String = s"${super.g(1, try 10 / n catch { case _: ArithmeticException => -1 })} ${super[T].h(1, try 10 / n catch { case _: ArithmeticException => -1 })}"
  def throws(n: Int): Int = try super.g(1, if n > 0 then throw new IllegalStateException("t") else n) catch { case _: IllegalStateException => -7 }
  def nested(n: Int): Int = super.g(1, super[T].h(2, if n > 0 then 3 else 4))
  def inLambda(n: Int): List[Int] = List(n, -n).map(m => super.g(1, if m > 0 then 2 else 3))
}

object O extends Base with T {
  def ifs(n: Int): Int = super.g(1, if n > 0 then 2 else 3) + super[T].h(1, n match { case 0 => 10; case _ => 20 })
}

object Main {
  def main(args: Array[String]): Unit = {
    val c = new C
    for n <- List(1, 0) do
      println(s"if $n: ${c.ifs(n)}")
      println(s"match $n: ${c.matches(n)}")
      println(s"try $n: ${c.tries(n)}")
      println(s"throw $n: ${c.throws(n)}")
      println(s"nested $n: ${c.nested(n)}")
      println(s"lambda $n: ${c.inLambda(n)}")
      println(s"object $n: ${O.ifs(n)}")
  }
}
