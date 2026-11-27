package lxi_obj_clause
object Lib {
  inline def inc(q: Int): Int = q + 1
  def value(x: Int): Int = {
    trait T { def m(a: Int)(b: Int = inc(a) + inc(x)): Int = a * 10 + b }
    class C extends T
    new C().m(2)()
  }
}
