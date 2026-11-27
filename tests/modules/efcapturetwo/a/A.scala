// A local trait's override default capturing two parameters of the enclosing method, a class mixing it in, the
// downstream leaving the argument: scalac prints 15.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int, j: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + k + j): Int = a + b }
    class C extends T
    new C
  }
}
