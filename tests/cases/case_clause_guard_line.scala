// A guard on its own line after a blank one.
def f(x: Any): Int = x match
  case 1

      if true => 1
  case _ => 0
@main def run(): Unit = println(f(1))
