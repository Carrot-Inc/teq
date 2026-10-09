package pvl

// A `PolyFunction` refinement with a `val` beside its `apply`, which dotty's
// `Checking.checkPolyFunctionType` leaves alone (it checks the method refinements).
object Api:
  def keep(f: PolyFunction { def apply[A](a: A): A; val tag: Int }) = f
