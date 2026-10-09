// expect: Use.scala:16:27: error: first expansion
// expect: 1 error found
// The error of a pending call's expansion run where the call is bound, inside an overload's
// alternative that fails, stays the call's when the alternative is given up: the second does not
// expand it again, successfully (scalac and master report it).
trait Ops:
  extension [T](x: Int)
    def combine(a: Int, b: String): Int = -1
  extension [T](x: Int)
    def combine(a: Int, b: Int): Int = x + a + b

object O extends Ops
def side(): Int = 10

@main def main(): Unit =
  println(O.combine[Unit](Counter.next)(b = side(), a = side()))
  println(Counter.next)
