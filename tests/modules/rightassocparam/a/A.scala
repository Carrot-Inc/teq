// An inline method's body binds the operand of a right-associative operator whatever its receiver: a parameter or
// `this` is a value where the body is typed and what the call gives it after the expansion, a lazy value here
// (`onList(lz)`: the operand first, then the lazy value's initializer). The whole build takes the binding back
// where the expansion's receiver does nothing (`onList(plain)`, `acc.pre()`); the build over this module's
// products keeps it, the pickled body declaring it as a value: the same order, another text (known.txt).
package lib
object Log:
  var s: String = ""
  def hit[A](tag: String, a: A): A = { s += tag; a }
  def take(): String = { val r = s; s = ""; r }
def left(): Int = Log.hit("l", 1)
lazy val lz: List[Int] = Log.hit("z", Nil)
inline def onList(xs: List[Int]): List[Int] = left() :: xs
class Acc(val items: List[Int]):
  inline def add(): List[Int] = left() :: items
  def +:(x: Int): String = "acc" + x
  inline def pre(): String = left() +: this
