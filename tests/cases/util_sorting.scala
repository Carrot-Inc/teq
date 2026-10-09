// scala.util.Sorting sorting arrays in place, stably, by an ordering or a comparison.
import scala.util.Sorting

object Main:
  def main(args: Array[String]): Unit =
    val a = Array(3, 1, 2)
    Sorting.stableSort(a)
    println(a.mkString(","))
    val b = Array("bb", "a", "cc", "d")
    Sorting.stableSort(b, (x: String, y: String) => x.length < y.length)
    println(b.mkString(","))
    val c = Array(5L, -1L)
    Sorting.quickSort(c)
    println(c.toList)
