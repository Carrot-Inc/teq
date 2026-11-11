// A call of a method declared to answer a literal type, in an inline argument: the argument stands as it was typed,
// the call kept (its type a constant one, the application not a pure one, so no literal stands for it).
class B:
  var calls = 0
  def k(x: Int): 4 = { calls += 1; 4 }
object M:
  val b = new B
  inline def once[T](inline x: T): T = x
  def main(args: Array[String]): Unit =
    println(once(b.k(1)))
    println(once(List(1, 2).map(b.k)))
    println(b.calls)
