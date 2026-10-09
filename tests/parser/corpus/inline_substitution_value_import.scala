// teq: --inline-substitution
// An import from a parameter, or one that names a value's member in the block, has no form in
// the stored body: the call takes the retype path, which finds the given scalac finds (7, 7).
import scala.compiletime.summonInline
class Values(val n: Int):
  given value: Int = n
inline def renamed(v: Values): Int =
  import v.{value as alias}
  summonInline[Int]
inline def givens(v: Values): Int =
  import v.given
  summonInline[Int]
@main def run(): Unit =
  println(renamed(new Values(7)))
  println(givens(new Values(7)))
