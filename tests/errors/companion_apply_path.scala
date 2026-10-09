// teq: --werror
// The inserted `apply`'s warning shows the companion's tree as scalac prints it
// (Typer.adaptNoArgsOther's `tree.show`, RefinedPrinter.toTextPrefixOf): its static path, the
// package included, whatever name reached it.
package cap
object Scope:
  case class C(n: Int)
import Scope.C as CC
case class D(n: Int)
def run() =
  val f: Int => Scope.C = CC
  val g: Int => Scope.C = Scope.C
  val h: Int => D = D
// expect: 11:27: warning: The method `apply` is inserted. The auto insertion will be deprecated, please write `cap.Scope.C.apply` explicitly.
// expect: 12:33: warning: The method `apply` is inserted. The auto insertion will be deprecated, please write `cap.Scope.C.apply` explicitly.
// expect: 13:21: warning: The method `apply` is inserted. The auto insertion will be deprecated, please write `cap.D.apply` explicitly.
