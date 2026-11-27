// A class local to a lazy val's initialiser in a local class, its method's override default, the downstream leaving
// it: scalac prints 22.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    class O {
      lazy val api: API = {
        class C extends API { override def m(a: Int)(b: Int = a + 8): Int = k + a + b }
        new C
      }
    }
    new O().api
  }
}
