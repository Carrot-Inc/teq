import scala.quoted.*

// A macro's lazy locals count in a local and an array of the run's own frame, made before their
// initialisers began and stamped by no initialiser's end: the run's own, which no other run
// reaches, so the runs stay on their workers.
object M:
  def count(e: Expr[Int])(using Quotes): Expr[Int] =
    val k = e.valueOrAbort
    var n = 0
    val counts = Array(0)
    lazy val a: Int = { n += 1; counts(0) += k; n * 100 + counts(0) }
    lazy val b: Int = { n += 1; counts(0) += k; n * 100 + counts(0) }
    Expr(a + b + a)

inline def mc(inline k: Int): Int = ${ M.count('k) }
