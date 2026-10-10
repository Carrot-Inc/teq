package p

// A package object's parents give its package their members, a def, a val, a type and an
// extension (dotty's `PackageClassDenotation.computeMembersNamed`: the package objects'
// non-private members but `Any`'s and `Object`'s), selected on the package, named inside it and
// imported with it over the products as in the whole build.

trait Base:
  def n: Int = 7
  val v: String = "v"
  type T = Int
  extension (i: Int) def twice: Int = i * 2

package object q extends p.Base:
  def own = 1
