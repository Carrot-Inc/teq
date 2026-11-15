// The operands of a right-associative operator with a repeated parameter and with two parameters are evaluated in
// their order ahead of the receiver: every element of a repeated parameter's sequence (the reference binds the
// sequence whole), and of two parameters' arguments each that is no pure expression.
class Rep:
  def +:(xs: Int*): Int = xs.sum
class Two:
  def +:(x: Int, y: Int): Int = x - y
def a: Int = { print("a"); 5 }
def b: Int = { print("b"); 2 }
def rep: Rep = { print("r"); new Rep }
def two: Two = { print("r"); new Two }
@main def run(): Unit =
  println((a, b) +: rep)
  println((a, b) +: two)
  println((a, 2) +: two)
  println((1, b) +: two)
