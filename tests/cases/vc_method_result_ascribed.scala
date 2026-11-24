// A value class's method whose result is ascribed to a wider type (`Any`, `Matchable`): the lowering leaves the call
// to the old code at the ascribed type, which writes the result as the underlying value and boxes it as the class
// (a value it left at another type than the one asked panicked the build before).
final class V(val u: Int) extends AnyVal {
  override def toString = s"V($u)"
  def twice: V = new V(u * 2)
}
object Main {
  def show(x: Any): String = x.toString
  def main(args: Array[String]): Unit = {
    println((new V(1).twice): Any)
    println(show((new V(2).twice): Any))
    val m: Matchable = (new V(3).twice: Matchable)
    println(m)
    println(((new V(4).twice): Any).getClass.getName)
  }
}
