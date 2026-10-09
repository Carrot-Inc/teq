// jars: scala-library
// std: scala-library
// scala-library's `Regex` and `ListSet` in link mode. The regular expressions run over the std's
// `java.util.regex`, which has an engine of its own rather than the lean std's `Regex` (that
// would call back into scala-library's); a `ListSet` node makes its successor as a member class
// of itself, `new Node(e)` under the node's own `this`.
import scala.collection.immutable.ListSet
import scala.util.matching.Regex

object Main:
  def main(args: Array[String]): Unit =
    val word = """(\w)(\w*)""".r
    println(word.findPrefixMatchOf("hello world").map(m => (m.matched, m.group(1), m.start(2), m.end(2))))
    println(word.findAllIn("one two three").toList)
    println(word.findAllMatchIn("ab cd").map(m => m.start + ":" + m.group(2)).mkString(","))
    println(word.replaceAllIn("big cat", m => m.group(1).toUpperCase + m.group(2)))
    println("a1b22c333".replaceAll("[0-9]+", "#"))
    val date = """(\d{4})-(\d{2})""".r
    "2024-02" match
      case date(y, m) => println(s"$y/$m")
      case _ => println("no")
    val p = java.util.regex.Pattern.compile("(x+)(y)?")
    val m = p.matcher("aaxxxb")
    println((m.find(), m.start(), m.end(), m.group(1), m.start(2), m.group(2)))
    val s = ListSet.from(List("at", "n", "at", "z"))
    println((s.size, s.contains("n"), s.contains("q"), s))
    println((ListSet.empty[Int] + 3 + 1 + 3 + 2).toList)
    println((s - "n").toList)
