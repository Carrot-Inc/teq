package lt_local_both_override_anon_first
object Lib:
  def value: String =
    val x = 10
    trait T { def m(a: Int = x + 3, b: Int): Int = a * 10 + b + x }
    val r = (new T { override def m(a: Int, b: Int): Int = a * 100 + b }).m(b = 1)
    r.toString
