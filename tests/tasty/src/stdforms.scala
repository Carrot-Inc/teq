package fix.stdforms

import scala.jdk.CollectionConverters.*

// The lean std's members scala-library has under another shape, each through the inverse table's
// form, the shape scala-library has for each member.
object StdForms:
  def charLit(s: String): Int = s.indexOf('|')
  def charVar(s: String, c: Char): Int = s.indexOf(c)
  def charBounded[C <: Char](s: String, c: C): Int = s.indexOf(c)
  def part(s: String): Int = s.indexOf("b")
  def words(s: String): List[String] = s.split(",").toList
  def sizes(l: List[Int]): Boolean = l.sizeIs > 1
  def sizesEq(l: List[Int]): Boolean = l.sizeIs == 2
  def flatOpt(o: Option[Option[Int]]): Option[Int] = o.flatten
  def nullable(o: Option[String]): String = o.orNull
  def scala(l: java.util.List[String]): List[String] = l.asScala.toList
  def range(n: Int): Int = (1 to n).foldLeft(0)(_ + _)
  def vals(m: Map[String, Int]): Int = m.values.toList.sum
  def dist(l: List[Int]): List[Int] = l.distinct
  def sorted(l: List[String]): List[String] = l.sorted(using Ordering.String)
  def triple(t: (Int, String, Boolean)): String = t match
    case (a, b, c) => s"$a$b$c"
  def makeTriple(a: Int): (Int, String, Boolean) = (a, "t", true)
