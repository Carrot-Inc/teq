// Patterns JDK 21's `Pattern` refuses and accepts, through `String.matches`: a counted closure whose maximum is below
// its minimum (`Pattern.closure`) and a group name defined twice (`Pattern.group0`) throw
// `PatternSyntaxException` on every target, as an unclosed class, an unknown escape and an unclosed closure do.
// (A possessive quantifier, `a++`, is left out: JavaScript's regex engine has none, docs/COMPATIBILITY.md.)
object Main:
  def main(args: Array[String]): Unit =
    for p <- List("[", "\\q", "a{", "a{2,1}", "(?<x>a)(?<x>b)", "(?<=a)b", "\\p{L}", "(?x)a # hi\nb", "a{2,}", "(?<x1>a)b") do
      try println(p.replace("\n", "~") + " => " + "ab".matches(p))
      catch case _: java.util.regex.PatternSyntaxException => println(p.replace("\n", "~") + " => syntax")
    try println(java.util.regex.Matcher.quoteReplacement(null))
    catch case _: NullPointerException => println("null npe")
