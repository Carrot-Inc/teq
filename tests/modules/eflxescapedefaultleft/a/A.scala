package lx_escape_default_left
trait API { def m(a: Int, b: Int = 6): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int, b: Int = x + 1): Int = a * 10 + b }
    class C extends T
    new C
  }
}
