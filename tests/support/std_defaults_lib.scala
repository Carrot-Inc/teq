// The jar of tests/cases/library_body_std_defaults.scala: bodies that leave out the defaults of the
// standard library's members, which scalac passes as the getters' applications on the receiver
// (`val $1 = it.flatMap(f); $1.indexWhere(p, $1.indexWhere$default$2)`).
package stddefaultslib

object Scan {
  def firstMatch(o: Option[String], word: String): Int =
    o.iterator.flatMap(s => s.split(' ').iterator).indexWhere(_ == word)
  def matchFrom(it: Iterator[Int], x: Int): Int = it.indexWhere(_ == x)
  def positionOf(xs: Seq[Int], x: Int): Int = xs.indexOf(x)
  def firstOver(xs: Vector[Int], n: Int): Int = xs.indexWhere(_ > n)
  def lastBelow(xs: Seq[Int], n: Int): Int = xs.lastIndexWhere(_ < n)
  def begins(xs: List[Int], prefix: List[Int]): Boolean = xs.startsWith(prefix)
  def firstUpper(s: String): Int = s.indexWhere(_.isUpper)
  def windows(xs: List[Int]): List[List[Int]] = xs.iterator.sliding(2).map(_.toList).toList
  def counted(n: Int): List[Int] = Iterator.range(0, n).toList
  def ranged(n: Int): List[Int] = Array.range(1, n).toList
}
