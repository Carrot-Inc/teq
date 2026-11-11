class T(k: Int):
  println(s"new T($k)")
  def h(x: Int = k): Int = x
  def m(a: Int)(b: Int = a + k): Int = b
@main def run(): Unit =
  println(T(1).h())
  println(new T(2).h())
  println(T(3).m(1)())
