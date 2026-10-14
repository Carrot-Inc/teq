//> using platform js
// `x.equals(y)` on a `Float` or `Double` receiver is the method of its own box (`Erasure.Boxing.box`, then
// `Object.equals`): a `Float` equals a boxed `Float` of its bits alone, a `Double` a boxed `Double`, so
// `1.0f.equals(1.0)` and `1.0.equals(1)` are false where `==` is true; `-0.0` is not `0.0`, a NaN equals itself.
// A receiver widened from an `Int` is a `Double` there. On JavaScript the numbers are one class, as under
// Scala.js (tests/jvm-expected has the JVM's answers).
class E:
  override def equals(a: Any): Boolean = false

object Main:
  def generic[A](a: A, b: Any) = a.equals(b)
  def widened(d: Double) = d.equals(1.0)
  def tparam[T](t: T) = t.equals(1.0)

  def main(args: Array[String]): Unit =
    val i = 1; val l = 1L; val f = 1.0f; val d = 1.0
    val a: Any = i; val r: AnyRef = java.lang.Integer.valueOf(1)
    println(List(i.equals(l), i == l, f.equals(d), d.equals(f), f == d))
    println(List(a.equals(d), r.equals(java.lang.Long.valueOf(1)), generic(i, l), generic(f, d)))
    println(List((-0.0).equals(0.0), -0.0 == 0.0, Double.NaN.equals(Double.NaN), Float.NaN.equals(Float.NaN)))
    println(List("a".equals("a"), new E().equals(1)))
    val x: Double = 3
    val fl: Float = 2
    println(List(x.equals(3.0), widened(1), tparam(1.0), tparam(1), List(1.0).head.equals(1.0)))
    println(List(fl.equals(2.0f), fl.equals(2.0), 1.0f.equals(1.0f), 1.0.equals(1), 1.0.equals(1L), 1.5.equals(BigDecimal(1.5))))
    val arr = Array(1.0, 2.0)
    println(List(arr(0).equals(1.0), (i: Double).equals(1.0), i.toDouble.equals(1.0), (i * 1.0).equals(1.0)))
