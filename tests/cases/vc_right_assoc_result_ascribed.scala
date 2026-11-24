// A generic value class's right-associative method, its result ascribed to `Any`: the underlying value is a type
// parameter's (`Object`), boxed as the class where `Any` is asked.
final class V[A](val u: A) extends AnyVal {
  override def toString = s"V($u)"
  def +:(k: Int): V[A] = new V(u)
}
object Main {
  def main(args: Array[String]): Unit = {
    println((3 +: new V("g")): Any)
    println(((4 +: new V(7)): Any) == new V(7))
  }
}
