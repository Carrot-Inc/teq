// A trailing `|` whose last alternative stands on the next line before the guard (dotty's `InCase` region).
def f(x: Any): Int = x match
  case 1 |
      2 if true => 1
  case _ => 0
@main def run(): Unit = println(f(1))
