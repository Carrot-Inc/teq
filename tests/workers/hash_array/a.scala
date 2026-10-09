import scala.quoted.*

// The identity hash of an array an object holds: its contents', the same on every
// worker's heap.
object M:
  val a = Array(0)
  def impl(using Quotes): Expr[Int] = Expr(System.identityHashCode(a))

inline def mh: Int = ${ M.impl }
