// A value class over a universal trait it extends: the underlying value and the box share the descriptor,
// and a parameter declared with the trait takes the box.
trait U extends Any { def f: Int }
class R extends U { def f: Int = 1 }
class V(val u: U) extends AnyVal with U { def f: Int = 2 }
object Main {
  def take(u: U): Int = u.f
  def id[A](x: A): A = x
  def main(args: Array[String]): Unit = {
    val v = new V(new R)
    println(v.f)
    println(take(v))
    val u: U = v
    println(u.f)
    println(u.getClass.getName)
    println(id[U](v).f)
  }
}
