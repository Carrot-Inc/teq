// A blank line after a case's trailing `|`: inside `InCase` no line break separates, blank or not.
def f(x: Any): Int = x match
  case 1 |

      2 => 1
  case _ => 0
@main def run(): Unit = println(f(1))
