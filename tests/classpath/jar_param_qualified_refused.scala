// jars: fixtures scala-library
// expect: cannot override val parameter value s in class PqA
// expect: s is private to PqA
// A library's `private[shapes]` constructor accessor (tests/tasty/src/paramqual.scala) from outside the package: a
// subclass's `override val` passed through a private alias inside it is no inherited accessor of it
// (`isAccessibleFrom(currentClass.thisType, superAccess = true)`, scalac 3.8.4 warns), and a selection of it is refused
// (scalac's E173, worded as teq words a source `private[p]`'s).
package fix.shapes {
  class PqB2(s: Int) extends PqA(s)
}

class PqC(override val s: Int) extends fix.shapes.PqB2(s)

object PqSelect:
  def read(a: fix.shapes.PqA) = a.s
