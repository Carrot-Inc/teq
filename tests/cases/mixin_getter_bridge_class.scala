// A default getter overriding a generic parent's in a class: the class holds the getter's bridge of the parent's
// erased signature, so a call through the parent's type reaches the subclass's default.
trait A[T] { def m(x: T)(y: T = x): T = y }
class C extends A[Int] { override def m(x: Int)(y: Int): Int = y }
class D extends C { override def m(x: Int)(y: Int = 7): Int = y }
object Main { def main(args: Array[String]): Unit = { val a: A[Int] = new D; println(a.m(2)()) } }
