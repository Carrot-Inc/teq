// The calls with defaults the writer lifts as scalac does: a constructor's
// second clause after an earlier argument, receivers a val, a lazy val, a var field and a
// captured parameter, an infix call on a block.
object DefaultsLiftedShapes {
  def mark(s: String): Int = { print(s); 0 }
  class K2(a: Int)(b: Int = mark("B"), c: Int, d: Int)
  class Box(val n: Int) { def f(a: Int, b: Int = n): Int = b }
  class KI { infix def f(a: Int, b: Int = mark("D")): Unit = () }
  var r3 = new Box(1)
  def second(): Unit = { new K2(mark("A"))(d = mark("D"), c = mark("C")); println() }
  def valReceiver(): Unit = { val r1: Box = new Box(1); println(r1.f(0)) }
  def lazyReceiver(): Unit = { lazy val r2: Box = { print("L"); new Box(1) }; println(r2.f(0)) }
  def varField(): Unit = println(r3.f({ r3 = new Box(2); 0 }))
  def closure(): Unit = { def captured(r: Box) = () => r.f(0); println(captured(new Box(1))()) }
  def blockInfix(): Unit = { ({ print("R"); new KI }) f mark("A"); println() }
  def main(args: Array[String]): Unit = { second(); valReceiver(); lazyReceiver(); varField(); closure(); blockInfix() }
}
