// A call that binds something ahead of a default's getter (an earlier clause's arguments, or its receiver) keeps its
// own type inside the block of those bindings: its result at a value class over Int is the value class, not the
// underlying Int (the JVM threw ClassCastException where scalac prints the values).
final class V(val u: Int) extends AnyVal { override def toString = s"V($u)" }

object Lib:
  def two[A](x: A)(y: A = x): A = y
  def const[A](x: A)(n: Int = 1): A = x
  def mid[A](x: A)(n: Int = 1)(z: A): A = z
  def last[A](x: A)(z: A)(y: A = z): A = y

class K:
  def one[A](x: A, n: Int = 1): A = x
  def first[A](n: Int = 1)(x: A): A = x
  def two[A](x: A)(y: A = x): A = y

object Main:
  def use(v: V): Int = v.u
  def main(args: Array[String]): Unit =
    def local[A](x: A)(y: A = x): A = y
    val k = new K
    val a: V = Lib.two(new V(1))()
    val b: V = Lib.const(new V(2))()
    val c: V = Lib.mid(new V(3))()(new V(4))
    val d: V = Lib.last(new V(5))(new V(6))()
    val e: V = local(new V(7))()
    val f: V = (new K).one(new V(8))
    val g: V = (new K).first()(new V(9))
    val h: V = k.two(new V(10))()
    val i: V = Lib.two[V](new V(11))()
    println(Seq(a, b, c, d, e, f, g, h, i).map(_.u).mkString(" "))
    println(use(Lib.two(new V(12))()) + use((new K).one(new V(13))))
    println((Lib.two(new V(14))(): Any))
    val s: String = Lib.two("s")()
    val n: Int = Lib.two(15)()
    println(s + " " + n)
