// A class implements the abstract methods of a Java interface without `override`, and
// overrides a default method with it (CharSequence.isEmpty).
final class Letters(n: Int) extends CharSequence, Comparable[Letters]:
  def length: Int = n
  def charAt(i: Int): Char = ('a' + i).toChar
  def subSequence(start: Int, end: Int): CharSequence = toString.substring(start, end)
  override def isEmpty: Boolean = n == 0
  def compareTo(that: Letters): Int = n - that.length
  override def toString: String = (0 until n).map(charAt).mkString

@main def run(): Unit =
  val cs: CharSequence = Letters(4)
  println(cs.length + " " + cs.charAt(2) + " " + cs.subSequence(1, 3) + " " + cs.isEmpty + " " + Letters(0).isEmpty)
  val c: Comparable[Letters] = Letters(3)
  println(c.compareTo(Letters(5)) + " " + List(Letters(2), Letters(1)).sortWith(_.compareTo(_) < 0))
