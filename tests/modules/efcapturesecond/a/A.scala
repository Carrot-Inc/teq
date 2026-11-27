// A local trait's capturing override default reached through a second local trait, a class mixing that one in, the
// downstream leaving the argument: scalac prints 12.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(k: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + k): Int = a + b }
    trait U extends T
    class C extends U
    new C
  }
}
