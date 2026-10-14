// A class that is a sequence and a function trait of the program's: called and tested as the function
// it is through either type, as scala-library's `Seq` and the trait declare it.
trait IntFunction extends (Int => Int)
final class Squares(n: Int) extends IndexedSeq[Int] with IntFunction:
  def apply(i: Int): Int = i * i
  def length: Int = n
  override def iterator: Iterator[Int] = Iterator.tabulate(n)(apply)
object Main:
  def run(f: IntFunction): Int = f(3)
  def main(args: Array[String]): Unit =
    val s = new Squares(4)
    println(run(s))
    val g: Int => Int = s
    println(List(1, 2).map(g))
    println(s.isInstanceOf[Int => Int])
