package sca

// A trait's super accessor of a generic parent's member whose type parameters bound one another,
// the parent's parameter at the end of the chain: the accessor erases each as the chain seen from
// the trait ends (`String`), whatever order the parameters are declared in.
trait Parent[A]:
  def f[B <: C, C <: A](x: B): B
  def g[C <: A, B <: C](x: B): C
  def h[B <: C, C <: D, D <: A](x: B, d: D): D

trait Stack extends Parent[String]:
  abstract override def f[B <: C, C <: String](x: B): B = super.f[B, C](x)
  abstract override def g[C <: String, B <: C](x: B): C = super.g[C, B](x)
  abstract override def h[B <: C, C <: D, D <: String](x: B, d: D): D = super.h[B, C, D](x, d)

class Base extends Parent[String]:
  def f[B <: C, C <: String](x: B): B = x
  def g[C <: String, B <: C](x: B): C = x
  def h[B <: C, C <: D, D <: String](x: B, d: D): D = d
