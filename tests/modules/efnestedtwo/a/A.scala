// A class nested two deep in a local class, its method's override default, the downstream leaving it: scalac prints 22.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    class O {
      class P {
        class C extends API { override def m(a: Int)(b: Int = a + 8): Int = k + a + b }
        def get(): API = new C
      }
      def get(): API = new P().get()
    }
    new O().get()
  }
}
