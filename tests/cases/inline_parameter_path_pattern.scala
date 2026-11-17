// A typed pattern of a class named through the path of an inline method's own parameter.
class Outer:
  class Box(val v: Int)
  class Sub(v: Int) extends Box(v)

inline def test(o: Outer)(x: Any): String = x match { case s: o.Sub => "sub"; case b: o.Box => "box"; case _ => "no" }

@main def run(): Unit =
  val o = new Outer
  println(test(o)(new o.Sub(1)) + test(o)(1))
