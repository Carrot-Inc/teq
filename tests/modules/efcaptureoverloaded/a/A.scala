// A local trait's capturing override default of an overloaded method, a class mixing it in, the downstream leaving
// the argument: scalac prints 12 then 3.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int; def m(s: String): Int }
object Lib {
  def make(k: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + k): Int = a + b; def m(s: String): Int = s.length }
    class C extends T
    new C
  }
}
