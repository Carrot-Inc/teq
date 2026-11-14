// A class that takes from its superclass a `compareTo(o: Any)` of another result type beside `Comparable`'s
// `compareTo` answers each by its own, directly and through either type: the lines scalac 3.8.4 prints.
class A { def compareTo(o: Any): String = "any" }
class B(val n: Int) extends A with Comparable[B] { def compareTo(b: B): Int = n - b.n }
object Main {
  def main(args: Array[String]): Unit = {
    val a = new B(1); val b = new B(3)
    println(a.compareTo(b))
    println(a.compareTo("x": Any))
    println((a: Comparable[B]).compareTo(b))
    println((a: A).compareTo(b))
  }
}
