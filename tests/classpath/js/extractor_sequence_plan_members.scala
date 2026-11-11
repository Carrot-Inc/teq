// The members a sequence pattern calls on an extractor's result that is no sequence: no length test where only a
// rest is matched, the extractor still called in an alternative, and a private `lengthCompare` passed over for the
// public `length`.
final class Counted:
  def lengthCompare(n: Int): Int =
    println("length")
    1 - n
  def apply(i: Int): Int = 42
  def drop(n: Int): Seq[Int] = Nil
  def toSeq: Seq[Int] = List(42)
object WithCount:
  def unapplySeq(x: Int): Counted = new Counted

final class Hidden:
  private def lengthCompare(n: Int): Int = -1
  def length: Int = 1
  def apply(i: Int): Int = 42
  def drop(n: Int): Seq[Int] = Nil
  def toSeq: Seq[Int] = List(42)
object WithHidden:
  def unapplySeq(x: Int): Hidden = new Hidden

final class Empty:
  def lengthCompare(n: Int): Int = -n
  def apply(i: Int): Int = 0
  def drop(n: Int): Seq[Int] = Nil
  def toSeq: Seq[Int] = Nil
object Called:
  def unapplySeq(x: Int): Empty =
    println("called")
    new Empty

def check(n: Int): Unit = n match
  case Called(_*) | 1 => println("matched")

@main def run(): Unit =
  0 match
    case WithCount(xs*) => println(xs)
  0 match
    case WithHidden(x) => println(x)
    case _ => println(-1)
  check(0)
