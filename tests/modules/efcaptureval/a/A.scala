// A local trait's override default capturing a local val, a class mixing it in, the downstream leaving the argument:
// scalac prints 20.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    val j = k * 2
    trait T extends API { override def m(a: Int)(b: Int = a + j): Int = a + b }
    class C extends T
    new C
  }
}
