// The lean std's members scala-library has under another shape: scalac's regeneration from teq's
// TASTy runs each form on the JVM.
import scala.jdk.CollectionConverters.*

object StdInverseForms:
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
  def main(args: Array[String]): Unit =
    println(StdInverseForms.charLit("a|b"))
    println(StdInverseForms.charVar("a|b", 'b'))
    println(StdInverseForms.charBounded("a|b", '|'))
    println(StdInverseForms.part("abc"))
    println(StdInverseForms.words("x,y"))
    println(StdInverseForms.sizes(List(1, 2)))
    println(StdInverseForms.sizesEq(List(1, 2)))
    println(StdInverseForms.flatOpt(Some(Some(3))))
    println(StdInverseForms.nullable(None) == null)
    val list = new java.util.ArrayList[String]()
    list.add("x")
    println(StdInverseForms.scala(list))
    println(StdInverseForms.range(3))
    println(StdInverseForms.vals(Map("a" -> 1)))
    println(StdInverseForms.dist(List(1, 1)))
    println(StdInverseForms.sorted(List("b", "a")))
    println(StdInverseForms.triple(StdInverseForms.makeTriple(1)))
