import ctx.A
@main def main(): Unit =
  println(A.f(using 1)(using "x"))
  println(A.g(using 2)(using "y"))
  println(A.h(using 3)(4)(using "z"))
