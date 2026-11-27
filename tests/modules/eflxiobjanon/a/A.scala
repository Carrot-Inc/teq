package lxi_obj_anon
object Lib {
  inline def inc(q: Int): Int = q + 1
  def value(x: Int): Int = {
    trait T { def m(a: Int, b: Int = inc(x)): Int = a * 10 + b }
    val t: T = new T {}
    t.m(2)
  }
}
