// A binder's `@` at the start of the next line of a case's pattern (dotty's `InCase`).
def f(x: Any): Int = x match
  case n
      @ 1 => 1
  case _ => 0
@main def run(): Unit = println(f(1))
