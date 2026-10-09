// teq: --std=scala-library
// expect: 13:43: error: Implementation restriction: PolyFunction apply must have exactly one parameter list and optionally type arguments. No by-name nor varags are allowed.
// expect: 14:60: error: PolyFunction only supports apply method refinements
// expect: 15:60: error: PolyFunction only supports apply method refinements
// The `PolyFunction` checks of a refined type deriving from it, through an intersection too, of its
// method refinements, a `var` its getter and setter (dotty's `Checking.checkPolyFunctionType`,
// `Desugar.refinedTypeToClass`); the exception for a generic `apply` is the parent's own symbol's
// (`Typer.typedRefinedTypeTree`), so the intersection's `apply` refines `Tag`'s. A `val` is left
// alone.
trait Tag:
  def apply[A](value: => A): Any
object Api:
  def inter(f: (Tag & PolyFunction) { def apply[A](value: => A): A }) = f
  def withVar(f: PolyFunction { def apply[A](a: A): A; var tag: Int }) = f
  def withDef(f: PolyFunction { def apply[A](a: A): A; def tag: Int }) = f
  def withVal(f: PolyFunction { def apply[A](a: A): A; val tag: Int }) = f
