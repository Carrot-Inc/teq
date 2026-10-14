// `String`'s `matches`, `split` and `replaceAll` compile their pattern as `Pattern.compile` does, so an
// invalid one throws `PatternSyntaxException` (the JDK's `String` delegates to `Pattern`); `split(regex)` takes
// one character but a metacharacter as text (the JDK's fast path), `"a.b".split(".")` splitting at every
// character, while scala-library's `split(c: Char)` and `split(Array[Char])` escape their characters.
import java.util.regex.PatternSyntaxException

object Main:
  def test(f: => Any): Unit =
    try println(f match { case a: Array[?] => a.mkString("[", "|", "]"); case x => x })
    catch
      case e: PatternSyntaxException => println(s"${e.getClass.getName} ${e.getPattern} ${e.getMessage.contains(e.getPattern)}")
      case e: Throwable => println(e.getClass.getName)

  def main(args: Array[String]): Unit =
    test("x".matches("["))
    test("x".split("["))
    test("x".split("[", 2))
    test("x".replaceAll("[", ""))
    test("x".replaceFirst("(", ""))
    test(java.util.regex.Matcher.quoteReplacement(null))
    test("x[y[z".split('['))
    test("a.b.c".split('.'))
    test("a.b.c".split("."))
    test("a.b.c".split("\\."))
    test("a|b|c".split("|"))
    test("a|b|c".split('|'))
    test("a,b;c".split(Array(',', ';')))
    test("a1b2c".split(Array('1', '2')))
    test("a,b,,".split(","))
    test("a,b,,".split(",", -1))
    test("a,b,c".split(",", 2))
    test("aXbXc".split('X'))
    test("a b  c".split("\\s+"))
    test("a\\b".split("\\\\"))
    test("x".split(Array.empty[Char]))
