class Ops:
  inline def f(inline xs: Int*): Int = xs.sum + xs.sum
class H(val o: Ops):
  export o.f
@main def run(): Unit =
  println(H(Ops()).f({ println("a"); 1 }, 2))
  println(Ops().f({ println("b"); 1 }, 2))
