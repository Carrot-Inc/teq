package lxi_obj_both_super
object Lib {
  inline def inc(q: Int): Int = q + 1
  def value(x: Int): Int = {
    trait T { def m(a: Int, b: Int = inc(x) + inc(3)): Int = x + a * 10 + b }
    class C extends T { override def m(a: Int, b: Int): Int = super.m(a) + 1 }
    new C().m(2, 0)
  }
}
