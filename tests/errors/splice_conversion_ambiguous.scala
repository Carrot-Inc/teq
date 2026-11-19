// expect: 10:35: error: Found:    X⏎Required: Seq[Int] | Array[? <: Int]⏎Note that implicit conversions cannot be applied because they are ambiguous;⏎both method seq and method arr convert from X to Seq[Int] | Array[? <: Int]
// expect: both method seq and method arr convert from X to Seq[Int] | Array[? <: Int]
// Two conversions that take a spliced sequence to a sequence and to an array of the elements are
// ambiguous for its adaptation to either (dotty's `typedWildcardStarArgExpr`), as scalac reports.
import scala.language.implicitConversions
class X
implicit def seq(x: X): Seq[Int] = Seq(7)
implicit def arr(x: X): Array[Int] = Array(8)
def f(xs: Int*): Int = xs.head
@main def run(): Unit = println(f(X()*))
