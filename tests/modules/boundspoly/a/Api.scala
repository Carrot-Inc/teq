package bnp

// A polymorphic function type whose bound names the method's parameter, in a call's inferred
// result: its bounds are the type's own.
trait Ctx:
  type T
object Api:
  def make(c: Ctx): [A <: c.T] => (a: A) => a.type = ???
  def inferred(c: Ctx) = make(c)
