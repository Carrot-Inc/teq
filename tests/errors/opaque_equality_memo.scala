// The primitive selection of `==` on an opaque type's value is made where it is
// typed: inside the alias's scope the type is `Int` and `x == 0` is `Int`'s;
// outside it is no `Int`, and the comparison with an `Int` is refused (E172),
// as scalac 3.8.4 reports it at `x` (L:C is where its caret begins, 1-based).
// expect: 13:32: error:
// expect: of types O.T and Int cannot be compared with == or !=
// expect: 1 error found
object O:
  opaque type T = Int
  def inside(x: T): Boolean = x == 0
  def mk(i: Int): T = i

def outside(x: O.T): Boolean = x == 0

@main def run(): Unit =
  println(O.inside(O.mk(0)))
  println(outside(O.mk(0)))
