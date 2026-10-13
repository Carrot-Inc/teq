package grra

// A trait's given at its type parameter, read over the products through the objects extending it:
// ranked downstream by its type as the object sees it (dotty's `Applications.compare`, `alt.widen`).
trait Base
class A extends Base
class B extends A
trait G[T <: Base]:
  def make: T
  given value: T = make
object O extends G[A]:
  def make = new A
