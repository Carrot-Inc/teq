package probe.impv
trait Same[A]:
  def same(a: A, b: A): Boolean
final case class Bag[T](items: List[T], pick: T)(using val eq: Same[T])
def count[T](bag: Bag[T]): Int =
  import bag.eq
  bag.items.count(x => eq.same(x, bag.pick))
object Main:
  def main(args: Array[String]): Unit =
    given Same[Int] = (a, b) => a == b
    println(count(Bag(List(1, 2, 1), 1)))
