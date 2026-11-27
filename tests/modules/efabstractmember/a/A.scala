package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { def m(a: Int)(b: Int = a + 8): Int }
    class C extends T { def m(a: Int)(b: Int): Int = 10 + a + b }
    new C
  }
}
