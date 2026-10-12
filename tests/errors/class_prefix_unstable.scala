// expect: 12:11: error: Outer is not a valid class prefix, since it is not an immutable path
// expect: 1 error found
// A nested class's constructor proxy applied through a def makes a `new` whose class prefix is no
// path, which dotty's PostTyper refuses (checkClassType with a stable prefix required, E083); one
// through a val is a path.
class Outer(x: Int):
  class Inner(y: Int)

object Test:
  def outer = Outer(1)
  val o = Outer(2)
  val x = outer.Inner(2)
  val y = o.Inner(3)
