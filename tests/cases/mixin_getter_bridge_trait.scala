// A trait's default getter overriding a generic parent's: the class mixing the trait in holds the getter's bridge,
// as a trait holds none, so a call through the parent's type reaches the trait's default.
trait A[T] { def m(x: T)(y: T = x): T = y }
trait B extends A[Int] { override def m(x: Int)(y: Int = 7): Int = y }
class C extends B
object Main { def main(args: Array[String]): Unit = { val a: A[Int] = new C; println(a.m(2)()); println(new C().m(2)()) } }
