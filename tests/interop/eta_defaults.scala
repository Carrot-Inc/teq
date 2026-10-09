// Not Scala: scalac expands `Dates.parse` to a function over both parameters and rejects it
// where a one-parameter function is expected; teq fills the trailing defaults, since a library
// writes a default where the JVM has an overload (`LocalDate.parse(text, formatter)`).
package etadefaults

object Dates:
  def parse(text: String, format: String = "iso"): String = s"$text/$format"
  def pad(text: String, width: Int = 4, fill: Char = '.'): String =
    text + fill.toString * (width - text.length)
  def scale(n: Int, by: Int = 10, plus: Int = 0)(tag: String = "t"): String = s"${n * by + plus}$tag"

class Parser(prefix: String):
  def read(s: String, strict: Boolean = false): String =
    if strict then prefix + s.trim else prefix + s

@main def main(): Unit =
  println(List("a", "b").map(Dates.parse))
  val g: String => String = Dates.parse
  println(g("z"))
  val h: (String, String) => String = Dates.parse
  println(h("x", "y"))
  println(List("ab", "c").map(Dates.pad))
  val two: (String, Int) => String = Dates.pad
  println(two("q", 2))
  val p = Parser("> ")
  println(List(" one", "two ").map(p.read))
  val s: Int => String => String = Dates.scale
  println(s(2)("x"))
