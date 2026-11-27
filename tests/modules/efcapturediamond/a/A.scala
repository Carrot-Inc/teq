// Two local traits each overriding the default with a capture, a class mixing both, the downstream leaving the
// argument: the last trait's default runs; scalac prints 13.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    trait L extends API { override def m(a: Int)(b: Int = a + k): Int = a + b }
    trait R extends API { override def m(a: Int)(b: Int = a + k + 1): Int = a + b }
    class C extends L with R
    new C
  }
}
