// A binding made for a right-associative operator's order is taken back after an expansion only where the call's
// receiver has become a pure path that cannot throw (an object without an initializer, the end of an inline
// recursion), one operand or two; beside a receiver the expansion makes an object with an initializer, a lazy
// value, a `def`, a `var` or a selection on a null prefix it stays, and so it does whatever its initializer
// becomes. Both operands' effects are logged.
package probe
object Log:
  var s: String = ""
  def hit[A](tag: String, a: A): A = { s += tag; a }
  def take(): String = { val r = s; s = ""; r }
trait Ops:
  def n: Int
  def +:(x: Int): String = "bag" + n + "+" + x
  def ++:(x: Int, y: Int): String = "bag" + n + "+" + x + "+" + y
  def +++:(x: Int)(using d: Dummy): String = "bag" + n + "+" + x + d.tag
class Bag(val n: Int) extends Ops
class Dummy(val tag: String)
given Dummy = new Dummy("!")
object Quiet extends Ops { def n: Int = 0 }
object Loud extends Ops { Log.hit("o", ()); def n: Int = 1 }
lazy val lazyBag: Ops = Log.hit("r", new Bag(2))
def defBag: Ops = Log.hit("r", new Bag(3))
var varBag: Ops = new Bag(4)
class Holder(val bag: Ops)
def left(): Int = Log.hit("l", 1)
def second(): Int = Log.hit("b", 2)
def assigning(): Int = { varBag = new Bag(9); Log.hit("l", 1) }
inline def one: Int = 1
inline def pick(inline k: Int): Ops = inline k match
  case 0 => Quiet
  case 1 => Loud
  case 2 => lazyBag
  case 3 => defBag
  case _ => varBag
inline def of(h: Holder): Ops = h.bag
// the receiver is a call still to be inlined where these bodies are typed
inline def cons(inline k: Int): String = left() +: pick(k)
inline def consAssigning(): String = assigning() +: pick(4)
inline def consSelected(h: Holder): String = left() +: of(h)
inline def consPure(inline k: Int): String = one +: pick(k)
inline def consTwo(inline k: Int): String = (left(), second()) ++: pick(k)
inline def consUsing(inline k: Int): String = left() +++: pick(k)
// an inline recursion that ends in a pure path
inline def all(inline n: Int): List[Int] = inline if n == 0 then Nil else Log.hit("l", n) :: all(n - 1)
