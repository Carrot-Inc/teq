package lx_escape_both_left
trait API { def m(a: Int, b: Int = 6): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int, b: Int = x + 1): Int = x + a * 10 + b }
    class C extends T
    new C
  }
}
