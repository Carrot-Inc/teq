package lt_local_body_override_anon_last
object Lib:
  def value: String =
    val x = 10
    trait T { def m(a: Int, b: Int = 2): Int = a * 10 + b + x }
    val r = (new T { override def m(a: Int, b: Int): Int = a * 100 + b }).m(1)
    r.toString
