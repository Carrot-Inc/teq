// A trailing `|` with no alternative after it before the guard: refused.
// expect: 5:7: error: expected a pattern
def f(x: Any): Int = x match
  case 1 |
      if true => 1
  case _ => 0
@main def run(): Unit = println(f(1))
