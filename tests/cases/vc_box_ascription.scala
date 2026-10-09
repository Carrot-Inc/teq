// A value class ascribed or cast to a type it conforms to is its box on the JVM: `Any`, `Matchable`, a
// universal trait, `this` in its own methods, a parameter bounded by it; toString and getClass are the box's.
trait U extends Any { def u: String = "U" + toString.takeWhile(_ != '@') }
class V(val x: Int) extends AnyVal {
  def asAny: Any = (this: Any)
  def castAny: Any = this.asInstanceOf[Any]
}
class S(val s: String) extends AnyVal
class D(val d: Double) extends AnyVal
class VU(val x: Int) extends AnyVal with U
class L(val l: Long) extends AnyVal { override def toString = s"L($l)" }
case class CV(x: Int) extends AnyVal
object Main {
  def any(a: Any): String = a.toString
  def mat(a: Matchable): String = a.toString
  def bounded[T <: V](t: T): Any = t
  def main(args: Array[String]): Unit = {
    val v = new V(1)
    println((v: Any)); println(v.asInstanceOf[AnyRef].getClass.getName); println((v: Matchable).toString)
    println(v.asInstanceOf[Any].getClass.getName)
    val a: Any = v; println(a.getClass.getName)
    println(any(v)); println(mat(v)); println(bounded(v).getClass.getName)
    println(v.asAny.getClass.getName); println(v.castAny.getClass.getName)
    println(v.toString); println(s"$v"); println("" + v); println(v.getClass.getName)
    println(new S("a").getClass.getName); println(new D(1.5).toString.takeWhile(_ != '@'))
    println(new L(2).toString); println((new L(2): Any).toString); println(new L(2))
    println(CV(1).toString); println((CV(1): Any).toString); println(CV(1).getClass.getName)
    val vu = new VU(3); println(vu.u); println((vu: U).u); println((vu: U).getClass.getName)
  }
}
