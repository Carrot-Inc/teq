// A value class's method called with its arguments named out of order, its result ascribed to `Any`, over a String:
// the call is the old code's, its result the underlying String boxed as the class where `Any` is asked.
final class V(val u: String) extends AnyVal {
  override def toString = s"V($u)"
  def mix(a: Int, b: Int): V = new V(u + a + b)
}
object Main {
  def main(args: Array[String]): Unit = {
    println((new V("a").mix(b = 2, a = 1)): Any)
    val xs: List[Any] = List((new V("b").mix(b = 4, a = 3)): Any)
    println(xs)
  }
}
