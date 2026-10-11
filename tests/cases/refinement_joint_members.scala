// A refined type member is the refinement's info met with the parent's (`Types.findMember`'s
// `goRefined`: `pinfo & rinfo`), so refining one name twice narrows it from both sides in either
// order; a member a refinement of one part of an intersection declares is the intersection's
// (`goAnd`), compared with the whole left side (`TypeComparer.secondTry` before `fourthTry`).
import compiletime.uninitialized

trait A { def a: String = "a" }
trait B { def b: String = "b" }
class C { type T }
type T1 = C { type T <: A } { type T <: B }
type U1 = C { type T <: B } { type T <: A }

abstract class D { type T; val xz: Any }
abstract class E { val name: String }

class Holder:
  var x: T1 = uninitialized
  var y: U1 = uninitialized
  def swap(): Unit =
    x = y
    y = x
  var p: E & D { val xz: T } = uninitialized
  var q: E & D { val xz: T } = uninitialized
  def copy(): Unit = q = p

def both(c: T1)(t: c.T): String = t.a + t.b

@main def run(): Unit =
  val h = Holder()
  h.swap()
  h.copy()
  val c = new C { type T = A & B }
  println(both(c)(new A with B {}))
  println(h.x == null)
