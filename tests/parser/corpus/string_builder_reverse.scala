// scala-library's `StringBuilder.reverse` reverses the chars, so a pair of surrogates comes out
// as its two halves in the other order, where `String.reverse` keeps the pair (the lean std's
// JVM builder is the JDK's, whose `reverse` keeps it too).
object Main:
  def units(s: String): List[Int] = s.toCharArray.map(_.toInt).toList
  def main(args: Array[String]): Unit =
    val g = "𝄞"
    println(units(new StringBuilder(g).reverse.toString))
    println(units(new StringBuilder("a" + g + "b").reverse.toString))
    println(new StringBuilder("abc").reverse.toString)
    println(units(new StringBuilder("\uDD1E" + "x" + "\uD834").reverse.toString))
