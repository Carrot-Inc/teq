//> using options -Werror
// teq: --werror
// `targetOfHiddenExtension` (RefChecks.scala:1204-1217) names the receiver's
// `typeSymbol`: for a method's own type parameter `T <: B` it is the bound's, and
// `TypeRef(C11, B).typeSymbol` is the alias `B` itself, not seen through (Types.scala:617-623).
// scalac 3.8.4 says "from type B" (and "from type BB" through an export forwarder's
// own type parameter). Waiting on the representation of a type parameter's bound.
// Reviewer #6's [3] and reviewer #7's [B] of the workers exports helper.
class Box(val n: Int):
  def size: Int = n
object C11:
  type B = Box
  extension [T <: B](t: T) def size: Int = 0
class Ops(val s: String):
  def size: Int = 0
object F2:
  type BB = Box
  extension [A <: BB](b: A)
    def ops: Ops = Ops(b.toString)
    export ops.size
@main def run(): Unit = ()
