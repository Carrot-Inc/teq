// jars: scala-library
// std: lean scala-library
// Patterns assembled at run time from strings, with class sets, comments mode and properties under
// `(?i)`, through matching, extraction, iteration, splitting and replacement, in the lean std and
// with scala-library's `Regex` over the std's `java.util.regex`: the groups' numbers and names,
// the UTF-16 positions, the callbacks' order and count and the replacement's expansion are
// JDK 24's, the lookaheads a set is written with on JavaScript taking no group.
import scala.util.matching.Regex

object Main:
  def main(args: Array[String]): Unit =
    val consonant = Seq("[a-z", "&&", "[^aeiou]]").mkString
    val r = ("(" + consonant + ")").r
    var calls = List.empty[String]
    println(r.replaceAllIn("bad", m => { calls = calls :+ m.matched; "$1$1" }))
    println(calls)
    println(r.replaceFirstIn("bad", "<$1>"))
    println(r.replaceAllIn("bad", Regex.quoteReplacement("$1")))
    var asked = List.empty[String]
    println(r.replaceSomeIn("bad", m => { asked = asked :+ m.matched; if m.matched == "b" then Some("bb") else None }))
    println(asked)
    val named = ("(?<c>" + consonant + ")(?<v>[aeiou])(" + consonant + ")?").r
    for m <- named.findAllMatchIn("banana tea bot") do
      println(s"${m.start} ${m.end} ${m.groupCount} ${m.group("c")} ${m.group("v")} ${m.group(3)}")
    val g = "\uD834\uDD1E"
    val wide = "[\\x{1D11E}[b]&&[^a]]".r
    println(wide.findAllMatchIn("a" + g + "b" + g).map(m => (m.start, m.end)).toList)
    println("a1b22c333d".split("[\\w&&[^a-z]]+").toList)
    println(("x" + g + "y" + g + "z").split("[" + g + "[y]]").toList.map(_.length))
    val spaced = ("(?x) ( " + consonant + " ) # a consonant\n [aeiou]").r
    println(spaced.findAllIn("banana").toList)
    println(spaced.findAllMatchIn("banana").map(_.group(1)).toList)
    println(spaced.replaceAllIn("banana", m => m.group(1).toUpperCase))
    val upper = ("(?i)" + "\\p{" + "Lu}+").r
    println(upper.findAllIn("abc 123 \u01c5x").toList)
    val date = ("(" + "[\\d&&[^9]]" + "{4})-(\\d{2})").r
    "2024-02" match
      case date(y, m) => println(s"$y/$m")
      case _ => println("no")
    "2094-02" match
      case date(y, m) => println(s"$y/$m")
      case _ => println("no")
    val p = java.util.regex.Pattern.compile("(" + consonant + ")(?<v>[aeiou])")
    val mm = p.matcher("xbe ca")
    val sb = new java.lang.StringBuilder
    while mm.find() do mm.appendReplacement(sb, "[$2$1]")
    mm.appendTail(sb)
    println(sb)
    println(p.matcher("be ca do").replaceAll("$1"))
    println("to be or not".replaceAll("[a-z&&[^aeiou]](?=[aeiou])", "_"))
    println("to be".replaceFirst("(?x) [a-z && [^aeiou]]", "#"))
    println("to be or not".split("(?x) \\  | [a-z && [^aeiou]] (?= \\ ) ").toList)
    val commented = java.util.regex.Pattern.compile("a b # comment", java.util.regex.Pattern.COMMENTS)
    println(s"${commented.matcher("ab").matches()} ${commented.matcher("a b").matches()}")
    println(java.util.regex.Pattern.compile(" [a-z && [^aeiou]] + ", java.util.regex.Pattern.COMMENTS | java.util.regex.Pattern.CASE_INSENSITIVE).matcher("StRNG").matches())
    println(java.util.regex.Pattern.compile("\\p{Lu}+", java.util.regex.Pattern.CASE_INSENSITIVE).matcher("ab\u01c5").matches())
