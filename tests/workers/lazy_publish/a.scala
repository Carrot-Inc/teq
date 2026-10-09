import scala.quoted.*

// A lazy val's initialiser assigns the object's other field an array it makes: the array is
// the object's state from then on, which the runs after change.
object M:
  var a: Array[Int] = null
  lazy val init: Unit = { a = Array(0) }
  def impl(using Quotes): Expr[Int] =
    init
    a(0) += 1
    Expr(a(0))

inline def mh: Int = ${ M.impl }
