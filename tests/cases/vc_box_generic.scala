// A value class as a type argument is its box: a generic method's argument and result, a List, a Map's keys
// and values, an Option, a tuple; and a class over a type parameter bounded by a universal trait it extends.
trait U extends Any { def f: Int }
class R extends U { def f: Int = 1 }
class V(val x: Int) extends AnyVal
class G[A](val u: A) extends AnyVal with U { def f: Int = 2 }
class B[A <: U](val u: A) extends AnyVal with U { def f: Int = 3; def inner: Int = u.f }
case class CV(x: Int) extends AnyVal
object Main {
  def id[T](t: T): T = t
  def pair[A, B](a: A, b: B): (A, B) = (a, b)
  def make[A](a: A): G[A] = new G[A](a)
  def take[A <: U](b: B[A]): Int = b.u.f
  def main(args: Array[String]): Unit = {
    val v = new V(1)
    println(id(v).getClass.getName); println(id[Any](v).getClass.getName); println(id(v).x)
    val p = pair(v, CV(2)); println(p); println(p._1.getClass.getName); println(p._2.x)
    val xs = List(new V(1), new V(2)); println(xs.map(_.getClass.getName)); println(xs.head.x); println(xs); println(xs.map(_.x).sum)
    val m = Map(new V(1) -> "one", new V(2) -> "two"); println(m(new V(2))); println(m.keys.toList.map(_.getClass.getName))
    val m2 = Map("a" -> new V(3)); println(m2("a").x); println(m2.values.head.getClass.getName)
    val ms: Map[String, Any] = Map("id" -> CV(9), "n" -> 9); println(ms("id")); println(ms("id") == 9); println(ms("id") == CV(9))
    val o = Option(new V(4)); println(o.map(_.x)); println(o.get.getClass.getName)
    val g = new G[U](new R); println(g.u.f); println(make[U](new R).u.f); println((g: U).f)
    val b = new B[R](new R); println(b.u.f); println(b.inner); println(take(b)); println((b: U).f)
    val f: B[R] => Int = x => x.u.f; println(f(b))
  }
}
