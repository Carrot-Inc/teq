def f(x: => Int = { println("default"); 5 }): Int = x + x
def g(n: Int, msg: => String = { println("msg default"); "m" }): String = msg + msg + n
def h(a: Int, b: => Int = 2)(c: => Int = a + b): Int = a + b + c

@main def run(): Unit =
  println(f())
  println(f({ println("given"); 1 }))
  println(g(1))
  println(g(2, "x"))
  println(h(1)())
  println(h(1)(10))
  println(h(1, 5)())
  println(scala.util.Try(assert(1 == 2)).isFailure)
  println(scala.util.Try(require(1 == 2, "nope")).isFailure)
  println(scala.util.Try(assert(1 == 1)).isSuccess)
