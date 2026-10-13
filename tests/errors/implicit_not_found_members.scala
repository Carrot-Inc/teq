// An `@implicitNotFound` text names the annotated class's type members as well as its parameters and its
// enclosing classes' (dotty's `formatAnnotationMessage` over `substitutableTypeSymbolsInScope`), each seen from
// the type searched for (`asSeenFrom(pt, sym)`): a refinement's alias, the projection where the member is
// abstract, the enclosing class's parameter through the instance a nested class is selected on.
import scala.annotation.implicitNotFound
@implicitNotFound("Missing ${Out}")
trait C:
  type Out
class Outer[A]:
  @implicitNotFound("No ${A} ${V} ${W}")
  trait E[W]:
    type V <: W
@implicitNotFound("Two ${V} ${W}")
trait E2[W]:
  type V <: W
val a = summon[C { type Out = String }]
val b = summon[C]
def f[T <: C](t: T) = summon[T]
val o = new Outer[Long]
val d = summon[o.E[Char]]
val e = summon[E2[Char] { type V = Char }]
val g = summon[E2[Char]]
// expect: implicit_not_found_members.scala:16:40: error: Missing String
// expect: implicit_not_found_members.scala:17:18: error: Missing C#Out
// expect: implicit_not_found_members.scala:18:32: error: Missing T#Out
// expect: implicit_not_found_members.scala:20:26: error: No Long o.E[Char]#V Char
// expect: implicit_not_found_members.scala:21:43: error: Two Char Char
// expect: implicit_not_found_members.scala:22:25: error: Two E2[Char]#V Char
