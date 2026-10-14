// `A with B` in a type, the deprecated intersection scalac 3.8.4 reads as `A & B` (dotty's `withType`): an
// operand of the infix types, binding tighter than `|` and associating to the right, in a cast, a parameter, a
// type argument and a parent's type argument; `with` between parents and before a body stays the template's.
trait Named { def name: String = "named" }
trait Sized { def size: Int = 2 }
trait Tagged { def tag: String = "#" }
class Item extends Named with Sized with Tagged

trait Box[T] { def get: T }
class ItemBox extends Box[Named with Sized]:
  def get: Named with Sized = new Item

def describe(x: Named with Sized with Tagged): String = x.tag + x.name * x.size
def either(x: Named with Sized | String): String = x match
  case s: String => s
  case n: Named => n.name

@main def main(): Unit =
  val any: Any = new Item
  val item = any.asInstanceOf[Named with Sized with Tagged]
  println(describe(item))
  println(either(item) + " " + either("plain"))
  val box = new ItemBox
  println(box.get.size)
  val items: List[Named with Sized] = List(new Item, new Item)
  println(items.map(_.size).sum)
