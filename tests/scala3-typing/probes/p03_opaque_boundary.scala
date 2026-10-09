opaque type A = String
val ok: A = "abc"
object obj:
  val y: A = "abc"
  def f(a: A): String = a
  def g(s: String): A = s
@main def run(): Unit = println(obj.y)
