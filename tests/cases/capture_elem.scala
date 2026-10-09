// A sequence where a function is expected is applied in a lambda whose parameter meets the
// definition that the sequence's inline `apply` reads.
def `elem$1`(): Int = 7

class Squares extends IndexedSeq[Int]:
  def length: Int = 3
  inline def apply(i: Int): Int = i * i + `elem$1`()

def at(f: Int => Int): Int = f(2)

@main def main(): Unit =
  println(at(Squares()))
