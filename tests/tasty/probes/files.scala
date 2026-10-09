// Which file holds what: a class with its companion, an object alone, a trait, top-level
// definitions of the file (`files$package`).
package probe.files

class Pair(val a: Int)
object Pair:
  def one: Pair = Pair(1)
object Alone:
  val x: Int = 1
trait Named:
  def name: String
def top(n: Int): Int = n
val topVal: String = "v"
type Alias = List[Int]
