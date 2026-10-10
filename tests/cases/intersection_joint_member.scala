// An intersection's member where neither part's result conforms to the other's: the symbol
// dotty's meet takes (the concrete one) with the results met (`Denotations.infoMeet`), for a
// value, a method, a polymorphic method and a generic part's member; a class that overrides
// both inherited members is checked against its own alone.
trait Concrete:
  def value: Any = "concrete"
  def twice(n: Int): Any = n * 2
  def pick[T](x: T): Any = x
trait Refined:
  def value: String
  def twice(n: Int): Int
  def pick[T](x: T): T
class Both extends Concrete with Refined:
  override def value: String = "both"
  override def twice(n: Int): Int = n + n
  override def pick[T](x: T): T = x
trait Holder[A]:
  def held: Any
trait Typed[A]:
  def held: A
class Box(x: List[Int]) extends Holder[List[Int]] with Typed[List[Int]]:
  def held: List[Int] = x

def read(x: Concrete & Refined): String = x.value
def doubled(x: Concrete & Refined): Int = x.twice(21)
def picked(x: Concrete & Refined): String = x.pick("p")
def first(x: Holder[List[Int]] & Typed[List[Int]]): Int = x.held.head

@main def main(): Unit =
  val b = new Both
  println(read(b))
  println(read(b).length)
  println(doubled(b) + 1)
  println(picked(b).toUpperCase)
  println(first(new Box(List(5, 6))))
