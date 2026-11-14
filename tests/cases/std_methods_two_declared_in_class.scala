// A class that declares both `compare` of `Ordered` and `compare` of `java.util.Comparator` answers each by its
// own, directly, through either type and through `<`: the lines scalac 3.8.4 prints.
class Ver(val n: Int) extends Ordered[Ver] with java.util.Comparator[Ver] {
  def compare(that: Ver): Int = n - that.n
  def compare(a: Ver, b: Ver): Int = b.n - a.n
}
object Main {
  def main(args: Array[String]): Unit = {
    val a = new Ver(1); val b = new Ver(3)
    println(a.compare(b))
    println(a.compare(a, b))
    println((a: Ordered[Ver]).compare(b))
    println((a: java.util.Comparator[Ver]).compare(a, b))
    println(a < b)
  }
}
