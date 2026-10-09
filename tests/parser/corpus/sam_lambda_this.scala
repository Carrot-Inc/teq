// `this` in a SAM lambda names the enclosing class, not the anonymous class the lambda becomes
// (cats' `Order.toOrdering` is `(x, y) => self.compare(x, y)` over `Ordering`).
trait Cmp[A]:
  def compare(x: A, y: A): Int
class Scaled(val k: Int):
  def compare(x: Int, y: Int): Int = (x - y) * k
  def cmp: Cmp[Int] = (x, y) => this.compare(x, y)
  def cmp2: Cmp[Int] = (x, y) => compare(x, y) + k
  def nested: Cmp[Int] = (x, y) =>
    val inner: Cmp[Int] = (a, b) => this.compare(a, b) * 10
    inner.compare(x, y) + this.k
object Main:
  def main(args: Array[String]): Unit =
    println(Scaled(2).cmp.compare(5, 2))
    println(Scaled(3).cmp2.compare(5, 2))
    println(Scaled(1).nested.compare(4, 1))
