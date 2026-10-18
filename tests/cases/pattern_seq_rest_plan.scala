//> using platform jvm
// A sequence pattern's rest as dotty's PatternMatcher.unapplySeqPlan binds it: an unapplySeq result's drop(n) as it
// returns it, null included, and the result's toSeq where no element comes before the rest (immutable over a
// mutable scrutinee).
final class Elements:
  def lengthCompare(n: Int): Int = 2 - n
  def apply(i: Int): Int = i + 1
  def iterator: Iterator[Int] = Iterator(1, 2)
  def toSeq: Seq[Int] = List(1, 2)
  def drop(n: Int): Seq[Int] =
    println("drop=" + n)
    null
object Extract:
  def unapplySeq(n: Int): Some[Elements] = Some(new Elements)
object Main:
  def main(args: Array[String]): Unit =
    0 match
      case Extract(head, rest*) => println(rest == null)
      case _ => println("no match")
    scala.collection.mutable.ArrayBuffer(1, 2) match
      case scala.collection.Seq(all*) => println(all.isInstanceOf[scala.collection.mutable.Buffer[?]].toString + " " + all.mkString(","))
      case _ => println("no")
