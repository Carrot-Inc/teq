//> using platform js
// `x.equals(y)` on a receiver typed `Any`, a primitive or a type parameter is the method, not
// `==`: a boxed number equals a box of its own class and value (the bits of a floating point:
// a NaN equals itself, -0.0 is not 0.0), so `1.equals(1L)` is false where `1 == 1L` is true; an
// object's own `equals` is called, on itself too; the receiver and the argument are evaluated
// once, in order. On JavaScript the numbers but Long are one class, as under Scala.js
// (tests/jvm-expected has the JVM's answers).
case class P(x: Int)
class Plain
class V(val n: Int) extends AnyVal

object Main:
  var calls = 0
  class E:
    override def equals(other: Any): Boolean = { calls += 1; true }
  def same[A](a: A, b: A): Boolean = a.equals(b)

  def main(args: Array[String]): Unit =
    val i: Any = 1
    val l: Any = 1L
    val d: Any = 1.0
    println(List(i.equals(l), i == l, i.equals(1), l.equals(1L), 1.equals(1L), 1L.equals(1)))
    println(List(i.equals(d), d.equals(i), (1: Short).equals(1), (1: Byte).equals(1: Byte), i.equals("1")))
    val n: Any = Double.NaN
    val z: Any = -0.0
    println(List(n.equals(Double.NaN), z.equals(0.0), z.equals(-0.0), n == Double.NaN, z == 0.0))
    val c: Any = 'a'
    println(List(c.equals('a'), c.equals(97), "ab".equals("a" + "b"), (true: Any).equals(true), (true: Any).equals(1)))
    val e: Any = new E
    println(List(e.equals(e), e.equals(1), calls))
    val p: Any = P(1)
    val pl: Any = new Plain
    println(List(p.equals(P(1)), p.equals(P(2)), pl.equals(pl), pl.equals(new Plain), same(P(3), P(3)), same(1, 1), same(1, 2)))
    println(List(new V(1).equals(new V(1)), new V(1) == new V(1), new V(1).equals(new V(2))))
    var order = ""
    def recv(): Any = { order += "r"; 2 }
    def arg(): Any = { order += "a"; 2 }
    println(List(recv().equals(arg()), order))
