package bnd

// A generic refinement member whose bound names the method's parameter, `A <: c.T`, forwarded:
// the bounds map with the member's types (dotty's `TypeMap.mapOverLambda`).
trait Ctx:
  type T
trait Parent[T]:
  def run[A <: T](a: A): Any
object Api:
  def keep(c: Ctx)(f: Parent[c.T] { def run[A <: c.T](a: A): A }) = f
