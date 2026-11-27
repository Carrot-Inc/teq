// A universal trait's method whose lambda returns `this`, called on a value class: the trait's code runs on the box,
// which the extension method makes for the call (scalac's `f$extension` makes the `V`), so the lambda keeps the box
// (`true`, then 5).
trait U extends Any {
  def thunk: () => Any = () => this
}
class V(val k: Int) extends AnyVal with U {
  def f: () => Any = thunk
}
object Main {
  def main(args: Array[String]): Unit = {
    val x = new V(5).f()
    println(x.isInstanceOf[V])
    println(x.asInstanceOf[V].k)
  }
}
