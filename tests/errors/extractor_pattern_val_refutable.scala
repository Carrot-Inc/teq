// expect: 7:28: warning: pattern's type String is more specialized than the right hand side expression's type Any
// teq: --werror
// An extractor whose result cannot fail makes no pattern val safe when a sub-pattern can fail,
// as scalac warns.
object Pair { def unapply(x: Any): (Any, Int) = (x, 1) }
def f(x: Any): Int =
  val Pair(s: String, n) = x
  n
