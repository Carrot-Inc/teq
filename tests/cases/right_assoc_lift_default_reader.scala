// An inline method's body binds a right-associative operator's argument (`typedInfixOp`'s lift), and its expansion
// takes the binding back beside a receiver whose evaluation can do nothing, where the argument's place is its one
// reader. A default's getter of a later clause takes the earlier clauses' arguments and reads the binding too: the
// block stays, all of its bindings.
class Box:
  def ++:(x: Int)(y: Int = mark("d")): Int = x + y
  def +++:(x: Int)(y: Int = x + mark("e")): Int = x + y
  def ++++:(x: Int, z: Int)(y: Int = x + mark("f")): Int = x + y + z
def mark(s: String): Int = { print(s); 1 }
inline def two(inline r: Box): Int = (mark("l") ++: r)()
inline def three(inline r: Box): Int = (mark("m") +++: r)()
inline def four(inline r: Box): Int = ((mark("n"), mark("o")) ++++: r)()

@main def main(): Unit =
  val b = new Box
  println(two(b))
  println(three(b))
  println(four(b))
