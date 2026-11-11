class Strict(k: Int):
  inline def e(inline x: Int = k): Int = x
  inline def f(inline x: Int = k)(y: Int = x * 2): Int = x + y
class Pub(val k: Int):
  inline def f(inline x: Int = k)(y: Int = x * 2): Int = x + y
class Sub(k: Int) extends Strict(k * 10)
class SubP(k: Int) extends Pub(k * 10)
class SubS(k: String) extends Strict(k.length * 10)
@main def run(): Unit =
  println(Sub(1).e())
  println(Sub(1).f()())
  println(SubP(1).f()())
  println(SubS("abc").e())
  println(SubS("abcd").f(5)())
