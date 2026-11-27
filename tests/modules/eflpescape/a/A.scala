package lp_escape
trait API { def m(a: Int, b: Int = 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int, b: Int = 8): Int = x + a + b }
    class C extends T
    new C
  }
}
object Caller { def call(t: API): Int = t.m(2) }
