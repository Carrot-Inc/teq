// A trailing `|` before the next case: refused, the next case parsed as its own.
// expect: 5:3: error: expected a pattern
def f(x: Int): Int = x match
  case 1 |
  case _ => 0
@main def run(): Unit = println(f(2))
