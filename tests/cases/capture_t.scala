// A lambda of two parameters where a function of a pair is expected takes the pair in a
// parameter named `t$0`, from which it binds its own.
def `t$0`(): Int = 7

@main def main(): Unit =
  println(List((1, 2)).map((a, b) => a + b + `t$0`()))
