package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + x): Int = 10 + a + b }
    class C extends T
    new C
  }
}
