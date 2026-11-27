package lp_escapeprior
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = x + a + b }
    class C extends T { override def m(a: Int)(b: Int): Int = super.m(a)(b) }
    new C
  }
}
