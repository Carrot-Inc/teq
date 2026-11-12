// Overload resolution's applicability test runs `tryDefault` for an
// omitted argument as the call does: the getter `DefaultGetterName(mk, 1)`
// by name (`findDefaultGetter`, Applications.scala:382-402), which
// the export of a case class's `apply` renamed `mk` does not have (its
// getters are `<init>`'s, Namer.scala:1249-1262). So that alternative
// is not applicable to `mk(y = 5)` and scalac 3.8.4 reports the other's
// "does not have a parameter y". Reviewer #7's [E] of the workers exports helper.
case class Pt(x: Int = 1, y: Int = 2)
object Z3:
  export Pt.{apply as mk}
  def mk(s: String): Pt = Pt(s.length, 0)
@main def run(): Unit =
  println(Z3.mk(y = 5))
