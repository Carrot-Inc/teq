// A Java source's declarations as the reference reads them: generics with a recursive bound, wildcards, a raw type,
// varargs, an imported member type of the JDK, the parameters' names, a mutable field, a protected member, an
// interface's static and default methods.
class Sub extends Box[String]("s") { def open: String = guarded() }
object Main {
  def main(args: Array[String]): Unit = {
    val a = Box.of(3)
    val b = Box.of(5)
    a.count = 4
    println(a.compareTo(b) < 0)
    println(Box.sum(1, 2, 3) + " " + Box.sum(Array(4, 5)*))
    println(Box.size(java.util.List.of(1, 2)) + " " + Box.rawSize(java.util.List.of("x")))
    println(Box.key(Box.entry("k", 1)))
    println(Box.scaled(by = 3, count = a.count))
    println(new Sub().open)
    println(Shape.unit().describe())
  }
}
