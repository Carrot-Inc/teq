// A pattern definition's element whose value erases to `Object` (a generic result, a tuple's `_2: T2`) read as
// the variable's primitive: unboxed, `null` its zero, as `Erasure.Boxing.adaptToType` adapts the definition
// (dotty's `makePatDef`): a lazy one's holder read, and a tuple's elements bound one by one.
object Main:
  def read[A](m: java.util.HashMap[String, A]): A = m.get("x")
  def main(args: Array[String]): Unit =
    val m = new java.util.HashMap[String, Int]()
    lazy val (x, (y, z)) = (read(m), (read(m), 2))
    println(y)
    println(y + z)
    lazy val (p, q) = (read(m), "s")
    println(p)
    println(q)
    val (a, b) = (read(m), 1)
    println(a)
    println(a + b)
    var (u, v) = (read(m), 4)
    println(u)
