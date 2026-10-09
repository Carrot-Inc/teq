// The imports a body's blocks make from a value, expanded by substitution: a name bound to a
// value's member (`import v.{m as a}`, which the record keeps as the selection it stands for,
// a given where the member is one) and an import from a parameter or a local (`import v.given`,
// which names the parameter's proxy or the local's copy in the expansion), each in scope from its
// statement to its block's end, where the expansion's searches find their givens. scalac prints
// the lines of the .expected file.
import scala.compiletime.summonInline
class Values(val n: Int):
  given value: Int = n
  val label: String = "L" + n
object Holder:
  val values = new Values(9)
inline def renamed(v: Values): Int =
  import v.{value as alias}
  summonInline[Int]
inline def givens(v: Values): Int =
  import v.given
  summonInline[Int]
inline def labelled(v: Values): String =
  import v.{label as l, value as g}
  l + " " + summonInline[Int]
inline def viaLocal(k: Int): Int =
  val local = new Values(k)
  import local.given
  summonInline[Int]
inline def nested(v: Values): Int =
  val out =
    import v.given
    summonInline[Int]
  out
inline def throughObject: Int =
  import Holder.values.given
  summonInline[Int]
@main def run(): Unit =
  println(renamed(new Values(7)))
  println(givens(new Values(7)))
  val w = new Values(8)
  println(labelled(w))
  println(viaLocal(5))
  println(nested(new Values(6)))
  println(throughObject)
