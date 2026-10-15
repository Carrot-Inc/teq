// A trailing `|` inside parentheses, where no line break separates anyway (dotty's `InParens`).
def f(x: Any): Int = x match
  case (1 |
      2) => 1
  case _ => 0
@main def run(): Unit = println(f(1))
