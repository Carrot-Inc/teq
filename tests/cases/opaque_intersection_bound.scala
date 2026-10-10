// An opaque type bounded by an intersection has, outside its scope, the members and the
// supertypes of both sides of the bound: dotty reads it there as an abstract type whose upper
// bound `findMember` and the subtype check go through.
trait Named { def name: String; def greet: String = s"hello $name" }
trait Sized { def size: Int; def name: String }
class Item(val name: String, val size: Int) extends Named with Sized

trait Source[T] { def items: List[T] }
trait Counted { def count: Int }
class Bag[T](val items: List[T]) extends Source[T] with Counted { def count = items.length }

object Store:
  opaque type Handle <: Named & Sized = Item
  def handle(name: String, size: Int): Handle = Item(name, size)
  opaque type Pack[T] <: Source[T] & Counted = Bag[T]
  def pack[T](xs: T*): Pack[T] = Bag(xs.toList)

def describe(n: Named): String = n.greet
def measure(s: Sized): Int = s.size
def both(x: Named & Sized): String = s"${x.name}:${x.size}"
def firstOf[T](s: Source[T]): T = s.items.head

@main def main(): Unit =
  val h = Store.handle("cup", 3)
  println(h.name)
  println(h.size)
  println(h.greet)
  println(describe(h))
  println(measure(h))
  println(both(h))
  val hs: List[Named] = List(h)
  println(hs.map(_.name))
  val p = Store.pack(1, 2, 3)
  println(p.items.map(_ + 1))
  println(p.count)
  println(firstOf(p))
