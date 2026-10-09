package nbd

// A generic member's bound naming the parameter of the refinement method it is the result of.
trait Ctx:
  type T
trait Parent[T]:
  def run[A <: T](a: A): Any
object Api:
  def keep(f: AnyRef { def outer(c: Ctx)(x: c.T): Parent[c.T] { def run[A <: c.T](a: A): A } }) = f
