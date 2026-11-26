package lt_local_body_override_anon_clause2
object Lib:
  def value: String =
    val x = 10
    trait T { def m(a: Int)(b: Int = a + 1): Int = a * 10 + b + x }
    val r = (new T { override def m(a: Int)(b: Int): Int = a * 100 + b }).m(1)()
    r.toString
