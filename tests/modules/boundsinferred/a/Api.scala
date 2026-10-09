package bni

// The same member in a call's inferred result, the bound's path the caller's parameter.
trait Ctx:
  type T
trait Parent[T]:
  def run[A <: T](a: A): Any
object Api:
  def make(c: Ctx): Parent[c.T] { def run[A <: c.T](a: A): A } = ???
  def inferred(c: Ctx) = make(c)
