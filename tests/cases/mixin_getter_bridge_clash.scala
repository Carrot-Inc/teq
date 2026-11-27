// A class declaring a method of its default getter's bridge's name and erased signature. Scala.js prints 7: there the
// method is an overload and the call takes C's default. scalac's JVM class does not load (two methods of that name
// and descriptor); on the JVM here the program's method stands where the bridge would be and the run prints 9, a
// departure tests/pending/jvm.txt lists (tests/cases holds no expectation of one target alone).
trait A[T] { def m(x: T)(y: T = x): T = y }
class C extends A[Int] {
  override def m(x: Int)(y: Int = 7): Int = y
  def `m$default$2`(x: Object): Object = (9: Any).asInstanceOf[Object]
}
object Main { def main(args: Array[String]): Unit = { val a: A[Int] = new C; println(a.m(2)()) } }
