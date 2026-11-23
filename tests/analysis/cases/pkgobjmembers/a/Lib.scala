package poma

// A package object's class, case class, val and given, members of the module `poma.inner.package`: the classes
// `poma.inner.package$Box` and `poma.inner.package$Point` (dotty's `Desugar.packageModuleDef`), which another module
// reads through an import of the package.
package object inner:
  class Box(val n: Int):
    def twice: Int = n * 2
  case class Point(x: Int, y: Int)
  val origin: Point = Point(0, 0)
  given unit: Box = new Box(1)
