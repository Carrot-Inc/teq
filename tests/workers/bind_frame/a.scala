import scala.quoted.*

// A run binds a local of a frame that an object's initialiser inside the run stamped: the
// closure over the frame reached the object before the local was bound, and the array the local
// takes is the run's, which the runs after change.
object Cache:
  var access: () => Array[Int] = null

object Other:
  val saved: () => Array[Int] = Cache.access

object M:
  def impl(using Quotes): Expr[Int] =
    if Cache.access == null then
      val current: Array[Int] =
        Cache.access = () => current
        val force = Other.saved
        Array(0)
    val data = Cache.access()
    data(0) += 1
    Expr(data(0))

inline def mh: Int = ${ M.impl }
