// The implicit scope of a class nested in a class holds its prefix (dotty's `addPath`): the
// prefix's givens, read on it, one candidate per prefix (`TermRefSet`); the class's companion
// through it, one per enclosing instance; and the scope of the prefix's type, its parents'
// companions included. A given of the enclosing class's `this` is no candidate for another
// instance's class.
trait TC[A]:
  def value: Int
class Parent
object Parent:
  given [A]: TC[A] with
    def value = 7

class O(val n: Int) extends Parent:
  class Item(val value: Int)
  given item: Item = new Item(n)
  def from(o: O): Int = summon[o.Item].value
  class Kept(val value: Int)
  object Kept:
    given kept: Kept = new Kept(n * 10)

@main def run(): Unit =
  val a = new O(1)
  val b = new O(2)
  println(summon[a.Item] eq a.item)
  type Alias = b.Item
  println(summon[Alias].value)
  println(a.from(b))
  println(summon[a.Kept].value)
  println(summon[b.Kept].value)
  println(summon[TC[a.Item]].value)
