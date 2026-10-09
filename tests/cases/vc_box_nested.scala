// A value class over a universal trait passed to another's constructor is its box: the argument takes the
// constructor parameter's representation (Erasure.scala 827), not the outer class's underlying one.
trait U extends Any { def f: Int }
class R extends U { def f: Int = 1 }
class V(val u: U) extends AnyVal with U { def f: Int = 2 }
class W[A](val u: A) extends AnyVal with U { def f: Int = 3 }
case class CU(u: U) extends AnyVal with U { def f: Int = 4 }
object Main {
  def main(args: Array[String]): Unit = {
    val inner = new V(new R)
    val outer = new V(inner)
    println(outer.u.f); println(outer.u.getClass.getName); println(outer.f)
    val w = new W[U](inner)
    println(w.u.f); println(w.u.getClass.getName)
    val c = CU(inner)
    println(c.u.f); println(c.u.getClass.getName); println(CU(outer).u.f)
    val deep = new V(new V(new V(new R)))
    println(deep.u.getClass.getName); println(deep.u.asInstanceOf[V].u.getClass.getName)
  }
}
