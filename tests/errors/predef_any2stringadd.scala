// expect: 11:19: error: value + is not a member of Any
// expect: 12:22: error: value + is not a member of AnyRef
// expect: 17:16: error: toString is not a member of the package
// expect: 3 errors found
// scala-library's deprecated `Predef.any2stringadd` is left out by the root import of `Predef`
// (dotty's `ImportInfo`: `rootImport` adds `any2stringadd as _`), so `x + "s"` on an `Any` is no
// member and no conversion; an explicit `import Predef.*` brings it back (`C`). A package has none
// of `Any`'s members (`checkImportSelectors`' `qualType.member`).
object A:
  def f(x: Any) = x.toString + "s"
  def g(x: Any) = x + "s"
  def h(o: Object) = o + "abc"
object C:
  import Predef.*
  def g(x: Any) = x + "s"
object D:
  import scala.toString
