package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait U { def k: Int = x }
    trait T extends API with U { override def m(a: Int)(b: Int = a + 8): Int = 10 + a + b }
    class C extends T
    new C
  }
}
