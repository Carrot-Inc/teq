// jars: scala-library
// A value class's box held as a reference goes to a destination that takes a reference as it is (dotty keeps a
// conforming tree, Erasure.scala 392-393, and unboxes only toward an ErasedValueType, 399-400): a null stays null,
// a box keeps its identity; only a destination that wants the underlying value unboxes it, a null to the zero.
class V(val u: Int) extends AnyVal
class S(val u: String) extends AnyVal
case class C(u: Int) extends AnyVal
object Main {
  var n = 0
  def missing[A]: A = { n += 1; null.asInstanceOf[A] }
  def take(v: V): Int = v.u
  def accept(x: Any): String = if (x == null) "null" else x.getClass.getName
  def probe(name: String)(f: => Any): Unit =
    try println(name + "=" + f) catch { case e: Throwable => println(name + "=" + e.getClass.getName) }
  def main(args: Array[String]): Unit = {
    probe("option") { Option(missing[V]) }
    probe("getClass") { missing[V].getClass.getName }
    probe("toString") { missing[V].toString }
    probe("case-toString") { missing[C].toString }
    probe("S-toString") { missing[S].toString }
    probe("interpolated") { s"${missing[V]}" }
    probe("accept") { accept(missing[V]) }
    probe("list") { accept(List(missing[V]).head) }
    probe("take") { take(missing[V]) }
    probe("equal-nulls") { missing[V] == missing[V] }
    val some = Some(new V(1))
    val a: Any = some.get
    val b: Any = some.get
    probe("identity") { a.asInstanceOf[AnyRef] eq b.asInstanceOf[AnyRef] }
    probe("same-box") { val l = List(new V(2)); l.head.asInstanceOf[AnyRef] eq l.head.asInstanceOf[AnyRef] }
    probe("count") { n }
  }
}
