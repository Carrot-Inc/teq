// The file's import statements are contexts of the package clause's level, in the order of the
// source: the later wildcard import of a given of the same name hides the earlier one.
package levels
object A:
  given Int = 1
object B:
  given Int = 2
import A.given
import B.given
@main def run(): Unit =
  println(summon[Int])
