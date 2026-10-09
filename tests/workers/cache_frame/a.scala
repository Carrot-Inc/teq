import scala.quoted.*

// A cacheable object's closure keeps an array a run makes in a local of its scope; the runs
// after change the array.
object Cache:
  val access: Boolean => Array[Int] =
    var current: Array[Int] = null
    (reset: Boolean) =>
      if reset then current = Array(0)
      current

object M:
  def impl(using Quotes): Expr[Int] =
    val a = Cache.access(false)
    val data = if a == null then Cache.access(true) else a
    data(0) += 1
    Expr(data(0))

inline def mh: Int = ${ M.impl }
