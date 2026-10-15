// A parenthesised union type across a line inside a typed pattern.
def f(x: Any): Int = x match
  case _: (Int |
      String) => 1
  case _ => 0
@main def run(): Unit = println(f(1))
