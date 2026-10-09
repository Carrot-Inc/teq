import scala.quoted.*

// An object's initialiser keeps an identity hash, which is its object's contents' whatever the runs
// before it hashed on the worker's heap.
class H

object State:
  val h: H = new H
  val id: Int = System.identityHashCode(h)

object M:
  def impl(n: Expr[Int])(using Quotes): Expr[Int] =
    var i = 0
    var noise = 0
    while i < n.valueOrAbort do
      noise = System.identityHashCode(new H)
      i += 1
    Expr(State.id + noise)

inline def mh(inline n: Int): Int = ${ M.impl('n) }
