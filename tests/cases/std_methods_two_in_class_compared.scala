// A class that inherits `compare` of `Ordered` from one trait and `compare` of `java.util.Comparator` from another
// answers each by its own, directly, through either type and through `<`: the lines scalac 3.8.4 prints.
class Ver(val n: Int)
trait ByOrder extends Ordered[Ver] { def n: Int; def compare(that: Ver): Int = n - that.n }
trait ByPair extends java.util.Comparator[Ver] { def compare(a: Ver, b: Ver): Int = b.n - a.n }
class Both(n: Int) extends Ver(n) with ByOrder with ByPair
object Main {
  def main(args: Array[String]): Unit = {
    val a = new Both(1); val b = new Both(3)
    println(a.compare(b))
    println(a.compare(a, b))
    println((a: Ordered[Ver]).compare(b))
    println((a: java.util.Comparator[Ver]).compare(a, b))
    println(a < b)
  }
}
