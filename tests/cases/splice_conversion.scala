// A spliced sequence is typed and adapted against a sequence or an array of the repeated parameter's
// elements (dotty's `typedWildcardStarArgExpr`), a conversion that takes it to either applied
// there and its result spliced as it is: an array as the array, a sequence as the sequence.
import scala.language.implicitConversions
class X
implicit def conv(x: X): Array[Int] = Array(7)
class Y
implicit def seqOf(y: Y): Seq[Int] = Seq(8, 9)
def f(xs: Int*): Int = xs.head
def g(xs: Int*): Int = xs.sum
@main def run(): Unit =
  println(f(X()*))
  println(g(Y()*))
