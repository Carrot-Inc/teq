// A partial function's `applyOrElse` takes the default in a parameter named `d$0`.
def `d$0`(): Int = 7

@main def main(): Unit =
  println(List(1, 2).collect { case 1 => `d$0`() })
