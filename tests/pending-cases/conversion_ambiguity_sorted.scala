// A view search for a member tries the conversions in `sort`'s order
// (`compareEligibles`, Implicits.scala:1584-1595): of one level and arity,
// the one whose owner has more base classes first, so scalac 3.8.4 names
// `c2` (of `T2`, which extends `Extra`) before `c1`. Queued: push 6,
// item 3 (the choice of a conversion through AmbiguousImplicits.qualify).
import scala.language.implicitConversions
class R
class HasPick:
  def pick: String = "h"
trait Extra
trait T1:
  given c1: Conversion[R, HasPick] = _ => new HasPick
trait T2 extends Extra:
  given c2: Conversion[R, HasPick] = _ => new HasPick
object M extends T1, T2:
  def run(): Unit = println((new R).pick)
object Main:
  def main(args: Array[String]): Unit = M.run()
