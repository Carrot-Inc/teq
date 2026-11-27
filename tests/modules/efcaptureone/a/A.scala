// A local trait's override default capturing the enclosing method's parameter, a class mixing it in, the downstream
// leaving the argument: scalac prints 12.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + k): Int = a + b }
    class C extends T
    new C
  }
}
