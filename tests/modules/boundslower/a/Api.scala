package bnl

// A generic refinement member's and a polymorphic function type's lower bounds naming the
// method's parameter, `A >: c.T`, under substitution (dotty's `TypeMap.mapOverLambda`).
trait Ctx:
  type T
trait Parent[T]:
  def run[A >: T](a: A): Any
object Api:
  def keep(c: Ctx)(f: Parent[c.T] { def run[A >: c.T](a: A): A }) = f
  def poly(c: Ctx): [A >: c.T] => (a: A) => a.type = ???
  def inferred(c: Ctx) = poly(c)
