package dpa

// Two members whose signatures differ (`f[T](x: T)` erases to `Object`, `f[U <: String](x: U)`
// to `String`): an intersection keeps both, the downstream's call takes the more specific.
trait C { def f[T](x: T): Any = "wrong" }
trait R { def f[U <: String](x: U): U }
class Both extends C with R { override def f[U <: String](x: U): U = x }
