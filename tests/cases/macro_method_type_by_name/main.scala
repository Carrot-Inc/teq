class C:
  def f(x: => Int, y: String): Int = x + y.length

@main def run =
  println(paramKinds(new C().f(1, "ab")))
