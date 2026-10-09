// A case class's `copy` holds the original in a local, which an argument of the copy reads
// past to the definition named as it is.
case class P(a: Int, b: Int)

def `c$0`(): Int = 7

@main def main(): Unit =
  println(P(1, 2).copy(b = `c$0`()))
