object O:
  inline def f(a: Int, inline xs: Int*): Int = a + xs.sum + xs.sum
  inline def g(inline xs: Int*)(y: Int = 1): Int = xs.sum + xs.sum + y
@main def run(): Unit =
  println(O.f(xs = { println("x"); 1 }, a = { println("a"); 1 }))
  println(O.g({ println("y"); 1 }, 2)())
