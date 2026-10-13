// An extension method defined in a given object's body, found through an import of the given:
// the candidate's members are its reference's type's, the module's own class (dotty's extension
// search over the givens in scope reads `P.ops.type`, not the declared `Ops`).
trait Ops
object P:
  given ops: Ops with
    extension (c: Int) def who = "ok"
    extension (s: String) def twice = s + s
object Q:
  given Ops with
    extension (b: Boolean) def flip = !b
@main def run(): Unit =
  import P.given
  println(1.who)
  println("ab".twice)
  import Q.given
  println(true.flip)
