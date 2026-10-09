// A pending call of an earlier list bound to a temporary where named arguments come out of order
// is expanded where it is bound, inside the first alternative of the overload, which fails on
// `b`: its expansion is the call's typing's and stays for the second, which does not run the
// macro again (scalac and master print 21 and 2).
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
