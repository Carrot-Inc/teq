// `java.util.Arrays.sort` with a comparator whose `compare` nothing else calls: the sort
// dispatches it at run time, so the output has to keep it (an `Ordering` is a comparator too).
import java.util.{Arrays, Comparator}

object Main:
  def main(args: Array[String]): Unit =
    val words = Array("pear", "fig", "banana")
    Arrays.sort(words, new Comparator[String] { def compare(a: String, b: String): Int = a.length - b.length })
    println(words.mkString(","))
    val names = Array("b", "c", "a")
    Arrays.sort(names, Ordering.String.reverse)
    println(names.mkString(","))
    val part = Array("e", "d", "c", "b", "a")
    Arrays.sort(part, 1, 4, Ordering.String)
    println(part.mkString(","))
