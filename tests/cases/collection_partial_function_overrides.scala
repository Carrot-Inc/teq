// A sequence's own applyOrElse and isDefinedAt, called through PartialFunction's type, and its apply through
// Function1's (scala-library's Seq is a PartialFunction of its indices).
final class Evens(n: Int) extends IndexedSeq[Int]:
  def apply(i: Int): Int = i * 2
  def length: Int = n
  override def applyOrElse[A1 <: Int, B1 >: Int](x: A1, default: A1 => B1): B1 =
    if isDefinedAt(x) then apply(x) + 1000 else default(x)
  override def isDefinedAt(i: Int): Boolean = i >= 0 && i < n
object Main:
  def main(args: Array[String]): Unit =
    val e = new Evens(3)
    val pf: PartialFunction[Int, Int] = e
    println(pf.applyOrElse(1, (_: Int) => -1))
    println(pf.applyOrElse(7, (_: Int) => -1))
    println(pf.isDefinedAt(2))
    println(pf(2))
    println(pf.lift(5))
    println(List(0, 1, 9).collect(pf))
    val f: Int => Int = e
    println(f.andThen(_ + 1)(1))
