object T:
  def a(n: Int) = if n == 0 then 0 else b(n - 1)
  def b(n: Int) = a(n - 1)
  def loop(n: Int) = if n == 0 then 0 else loop(n - 1)
@main def run(): Unit = println(T.a(3))
