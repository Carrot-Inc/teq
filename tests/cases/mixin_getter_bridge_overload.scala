// A class declaring a method of its default getter's bridge's name with another descriptor: the bridge is still
// written beside it, and the call through the parent's type takes the class's default.
trait A[T] { def m(x: T)(y: T = x): T = y }
class C extends A[Int] {
  override def m(x: Int)(y: Int = 7): Int = y
  def `m$default$2`(x: String): Object = (9: Any).asInstanceOf[Object]
}
object Main { def main(args: Array[String]): Unit = { val a: A[Int] = new C; println(a.m(2)()) } }
