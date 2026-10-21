// A binder's bound whose annotation's argument names the binder refers back to it (`checkNonCyclic` reads the
// annotation's tree), E140, as scalac 3.8.4 refuses it. teq drops a type's annotation other than `@RuntimeChecked`
// without typing its arguments (master's), so it accepts the bound; a part of its own (found by the fold of the review of
// `6c316c160`, probing `isNullType`'s reads under a binder's completion). scalac's line, at its tree's point.
// expect: 8:8: error: illegal cyclic type reference: upper bound Int @ann(Option.empty[T]) of type T refers back to the type itself
class ann(x: Any) extends scala.annotation.StaticAnnotation
object CycleOnly {
  def f[T <: Int @ann(Option.empty[T])](x: T): T = x
}
