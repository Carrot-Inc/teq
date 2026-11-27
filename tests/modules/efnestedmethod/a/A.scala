// A class local to a method of a local class, its method's override default, the downstream leaving it: scalac
// prints 22.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    class O {
      def get(): API = {
        class C extends API { override def m(a: Int)(b: Int = a + 8): Int = k + a + b }
        new C
      }
    }
    new O().get()
  }
}
