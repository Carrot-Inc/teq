object B:
  def f(x: Int = A.inferred): Int = x + 1
@main def run(): Unit = println(B.f())
