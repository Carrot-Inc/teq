package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = 10 + a + b }
    trait T2 extends T
    class C extends T2
    new C
  }
}
