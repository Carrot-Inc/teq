// A generator's wildcard is a parameter named `w$0`.
def `w$0`(): Int = 7

@main def main(): Unit =
  println(for _ <- List(1, 2) yield `w$0`())
