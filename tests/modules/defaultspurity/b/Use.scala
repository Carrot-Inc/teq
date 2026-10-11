package dpuruse

import dpur.*

def mark(s: String): Int = { print(s); 1 }

@main def main(): Unit =
  println(Api.f(b = Inits, a = mark("a"))())
  println(Api.f(b = Plain, a = mark("b"))())
