// A sequence pattern over an extractor's result that is a wrapper of no sequence type, as scala-library's
// `UnapplySeqWrapper`s are: the length, the elements and the rest are what its lengthCompare, apply, drop and toSeq
// give.
final class Slots(val values: List[Int]) extends AnyVal:
  def isEmpty: Boolean = false
  def get: Slots = this
  def lengthCompare(n: Int): Int = values.lengthCompare(n)
  def apply(i: Int): Int = values(i)
  def drop(n: Int): Seq[Int] = values.drop(n)
  def toSeq: Seq[Int] = values

object Slots:
  def unapplySeq(s: String): Slots = new Slots(s.split(",").toList.map(_.trim.toInt))

@main def run(): Unit =
  "1, 2, 3" match
    case Slots(a, b, rest*) => println(s"$a $b $rest")
  "4" match
    case Slots(all*) => println(all)
  "5, 6" match
    case Slots(x) => println("one " + x)
    case Slots(x, 6) => println("two " + x)
  "7, 8" match
    case Slots(_, y) => println("second " + y)
  "9" match
    case Slots(x, y) => println("two")
    case _ => println("none")
