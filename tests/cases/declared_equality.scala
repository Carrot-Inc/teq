// A class's own `==` overloads `Any`'s and is picked where it takes the operand: scala-library's
// `SizeCompareOps` of `sizeIs`, and a program's, one with a using clause after the operand's too.
final class Meters(val v: Int):
  def ==(n: Int): Boolean = v == n

@main def run(): Unit =
  val xs = Set(1, 2)
  println(xs.sizeIs == 1)
  println(List(1).sizeIs == 1)
  println(xs.sizeIs != 2)
  println(xs.sizeIs > 1)
  val m = Meters(3)
  println(m == 3)
  println(m == m)
  contextual()

class WithContext:
  def ==(i: Int)(using flag: Boolean): Boolean = flag

def contextual(): Unit =
  given Boolean = true
  println(new WithContext == 1)
