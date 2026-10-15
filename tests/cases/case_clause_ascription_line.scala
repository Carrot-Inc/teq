// A typed pattern's `:` at the start of the next line.
def f(x: Any): Int = x match
  case n
      : Int => 1
  case _ => 0
@main def run(): Unit = println(f(1))
