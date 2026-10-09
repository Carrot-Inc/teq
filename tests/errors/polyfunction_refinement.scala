// teq: --std=scala-library
// expect: 10:36: error: Implementation restriction: PolyFunction apply must have exactly one parameter list and optionally type arguments. No by-name nor varags are allowed.
// expect: 11:38: error: Implementation restriction: PolyFunction apply must have exactly one parameter list and optionally type arguments. No by-name nor varags are allowed.
// expect: 12:38: error: Implementation restriction: PolyFunction apply must have exactly one parameter list and optionally type arguments. No by-name nor varags are allowed.
// The `PolyFunction` refinements a source writes are checked as the ones `[A] => ..` desugars to
// (dotty's `Checking.checkPolyFunctionType`, `Definitions.isValidPolyFunctionInfo`): one parameter
// list, no by-name or repeated parameter; a monomorphic `apply` is one. scala-library's std, where
// `scala.PolyFunction` is.
object Api:
  def byName(f: PolyFunction { def apply[A](value: => A): A }) = f
  def twoLists(f: PolyFunction { def apply[A](a: A)(b: A): A }) = f
  def repeated(f: PolyFunction { def apply[A](as: A*): A }) = f
  def mono(f: PolyFunction { def apply(a: Int): Int }) = f
  def fine(f: PolyFunction { def apply[A](a: A): A }) = f
