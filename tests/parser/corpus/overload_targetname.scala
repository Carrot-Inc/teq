//> using scala 3.8.4
import scala.annotation.targetName
object Totals:
  @targetName("totalInts")
  def total(xs: List[Int]): Int = xs.sum
  @targetName("totalStrings")
  def total(xs: List[String]): Int = xs.map(_.length).sum
@main def run(): Unit =
  println(Totals.total(List(1, 2, 3)))
  println(Totals.total(List("ab", "c")))
