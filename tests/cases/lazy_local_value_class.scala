// A local lazy val in a value class's method, whose body its companion holds (`m$extension`, `this` the underlying
// value), with an anonymous class made there,
// the extension's own reads and lambdas.
import java.util.concurrent.Callable
class V(val u: Int) extends AnyVal {
  def go(): Int = { lazy val x = u + 1; new Callable[Int] { def call(): Int = x }.call() }
  def twice(): Int = { lazy val y = u * 2; y + new Callable[Int] { def call(): Int = y }.call() }
  def viaLambda(): Int = { lazy val z = u + 3; List(1).map(_ => z).head }
}
class W(val s: String) extends AnyVal {
  def go(): String = { lazy val x = s + "!"; new Callable[String] { def call(): String = x }.call() }
}
object Main:
  def main(args: Array[String]): Unit =
    println(List(new V(41).go(), new V(10).twice(), new V(4).viaLambda()))
    println(new W("hi").go())
