// teq: --std=scala-library
// expect: 16:13: error: type mismatch: found Any, required AnyVal. Note that implicit conversions were not tried because the result of an implicit conversion must be more specific than AnyVal
// expect: 17:17: error: type mismatch: found Any, required AnyVal. Note that implicit conversions were not tried because the result of an implicit conversion must be more specific than AnyVal
// expect: 18:17: error: type mismatch: found Any, required AnyRef. Note that implicit conversions were not tried because the result of an implicit conversion must be more specific than AnyRef
// expect: 3 errors found
// The second round of the overload resolution takes `f(x: AnyVal)` for an `Any` through the
// view Predef's value classes give (`viewExists`), but the argument is then adapted as any
// value is: no conversion is tried to `AnyVal` or `AnyRef` itself (`Typer.adapt`'s
// `TooUnspecific`), so the call is a mismatch, as scalac reports it (E007), at the argument,
// where the argument typed ahead is adapted at its own tree.
object E:
  def f(x: String): String = "string"
  def f(x: AnyVal): String = "val"

val x: Any = 1
val y = E.f(x)
val v: AnyVal = x
val r: AnyRef = x
