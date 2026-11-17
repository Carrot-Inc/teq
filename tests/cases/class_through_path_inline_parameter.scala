// The consumers of a class named through the path of an inline method's own parameter: its own `==`, its array,
// its tag, and a typed pattern of it.
import scala.reflect.{ClassTag, classTag}

class Outer:
  class Box(val v: Int) { def ==(o: Box): Boolean = v == o.v }
  class Sub(v: Int) extends Box(v)

inline def same(o: Outer)(a: o.Box, b: o.Box): Boolean = a == b
inline def make(o: Outer)(n: Int): Array[o.Box] = new Array[o.Box](n)
inline def tag(o: Outer): ClassTag[o.Box] = classTag[o.Box]
inline def test(o: Outer)(x: Any): String = x match { case s: o.Sub => "sub"; case b: o.Box => "box"; case _ => "no" }

@main def run(): Unit =
  val o = new Outer
  println(same(o)(new o.Box(1), new o.Box(1)))
  val a = make(o)(2)
  a(0) = new o.Box(1)
  println(a(0).v + a.length)
  println(tag(o) == classTag[o.Box])
  println(test(o)(new o.Sub(1)) + test(o)(1))
