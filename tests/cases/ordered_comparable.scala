// A class that is `Ordered` is `Comparable` too, as scala-library declares it.
final class Money(val n: Int) extends Ordered[Money]:
  def compare(that: Money): Int = n.compareTo(that.n)
  override def toString = s"Money($n)"
object Sorter:
  def largest[A <: Comparable[A]](xs: List[A]): A = xs.reduceLeft((a, b) => if a.compareTo(b) >= 0 then a else b)
  def byComparable[A <: Comparable[A]]: Ordering[A] = (a, b) => a.compareTo(b)
object Main:
  def main(args: Array[String]): Unit =
    val xs = List(Money(3), Money(1), Money(2))
    println(Sorter.largest(xs))
    val ord: Ordering[Money] = Sorter.byComparable
    println(xs.sorted(using ord))
    val c: Comparable[Money] = Money(4)
    println(c.compareTo(Money(5)))
    println(Money(1) < Money(2))
