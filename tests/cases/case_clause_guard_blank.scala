// A guard continued by a leading operator after a blank line, inside the case's `InCase` region.
def f(x: Int): Int = x match
  case 1 if true

    && true => 1
  case _ => 0
@main def run(): Unit = println(f(1))
