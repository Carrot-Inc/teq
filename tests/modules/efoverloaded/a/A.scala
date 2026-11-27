package esc
trait API { def m(a: Int)(b: Int = a + 7): Int; def m(s: String): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = 10 + a + b; def m(s: String): Int = s.length }
    class C extends T
    new C
  }
}
