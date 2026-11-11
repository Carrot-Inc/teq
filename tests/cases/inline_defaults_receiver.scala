class S(k: Int):
  println(s"new S($k)")
  inline def f(inline x: Int = { println("sx"); k })(y: Int = x * 2): Int = y
  inline def g(x: Int = k + 1): Int = x
@main def run(): Unit =
  println(S(6).f()())
  println(S(7).g())
  println(S(8).f(1)())
