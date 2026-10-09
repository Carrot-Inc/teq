import scala.quoted.*

// A run puts an array into a cacheable object's slot, then an object's initialiser runs inside
// the run: the array is the run's, whose changes the runs after make.
object Cache:
  val slot = new Array[Array[Int]](1)

object Other:
  val x: Int = 1

object M:
  def impl(using Quotes): Expr[Int] =
    if Cache.slot(0) == null then
      Cache.slot(0) = Array(0)
      val force = Other.x
    val a = Cache.slot(0)
    a(0) += 1
    Expr(a(0))

inline def mh: Int = ${ M.impl }
