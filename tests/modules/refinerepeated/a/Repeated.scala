package rra

trait Parent:
  def rep(xs: Int*): Int
  def gen[A]: Any

// A refinement's repeated parameter, `<repeated>[Int]` in its method type, and a generic member
// of no parameter list, the `POLYtype` over its result.
object Repeated:
  def keep(f: Parent { def rep(xs: Int*): Int; def gen[A]: A }) = f
