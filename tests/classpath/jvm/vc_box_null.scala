// jars: scala-library
// A null where a value class's underlying value is wanted unboxes to the underlying type's zero, the
// expression evaluated once; the class's parameter selected on a null throws.
class V(val u: Int) extends AnyVal
class S(val u: String) extends AnyVal
object Main {
  def probe(name: String)(f: => Any): Unit =
    try { println(name + "=" + f) } catch { case e: Throwable => println(name + "=" + e.getClass.getName) }
  var n: Int = 0
  def missing[A]: A = { n += 1; null.asInstanceOf[A] }
  def missingOf[A](x: Int): A = null.asInstanceOf[A]
  def take(v: V): Int = v.u
  def takes(v: S): String = v.u
  def main(args: Array[String]): Unit = {
    probe("pass-null-V") { take(missing[V]) }
    probe("pass-null-S") { takes(missing[S]) }
    probe("count") { n }
    probe("local-null-V") { val v: V = missing[V]; v.u }
    probe("ascribed-null") { val a: Any = null; take(a.asInstanceOf[V]) }
    probe("typed-null-accessor") { missing[V].u }
    probe("nested-accessor") { missingOf[V](new V(1).u).u }
    probe("block-accessor") { { n += 0; missing[V] }.u }
    probe("if-accessor") { (if (n > 0) missing[V] else new V(2)).u }
    probe("match-accessor") { (n match { case 0 => new V(3); case _ => missing[V] }).u }
  }
}
