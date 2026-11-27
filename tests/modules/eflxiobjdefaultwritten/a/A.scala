package lxi_obj_default_written
object Lib {
  inline def inc(q: Int): Int = q + 1
  def value(x: Int): Int = {
    trait T { def m(a: Int, b: Int = inc(x) + inc(3)): Int = a * 10 + b }
    class C extends T
    new C().m(2, 3)
  }
}
