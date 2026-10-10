// The constant-result exemption of an unused parameter (`CheckUnused.isUnconsuming`): a body whose
// type is a constant type, a block's being its result's, a call of a method of a literal result
// type whatever it does (`k`); an ascription widens it (`j`).
object Main {
  def a(x: Int): 1 = { println("ok"); 1 }
  def b(x: Int): Int = { println("ok"); 1 }
  def c(x: Int) = { println("ok"); 1 }
  def d(x: Int): Int = { val y = 2; 1 }
  def e(x: Int): String = { println(); "s" }
  final val K = 3
  def f(x: Int): Int = { println(); K }
  def g(x: Int): Int = { println(); 1 + 2 }
  def h(x: Int): Any = { println(); 1 }
  def i(x: Int): Int = if (true) 1 else 2
  def j(x: Int): Int = { println(); (1: Int) }
  def one(i: Int): 1 = { println(i); 1 }
  def k(x: Int): Int = { println("ok"); one(2) }
  def main(args: Array[String]): Unit = println(Seq[Any](a(0), b(0), c(0), d(0), e(0), f(0), g(0), h(0), i(0), j(0), k(0)))
}
