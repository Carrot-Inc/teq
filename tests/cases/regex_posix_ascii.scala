// Java's POSIX classes `\p{Lower}`, `\p{Upper}` and `\p{Alpha}` are of ASCII characters alone,
// `\p{Lower}` and `\p{Upper}` of either case under CASE_INSENSITIVE, where `\p{Ll}` and `\p{L}` are
// Unicode's; and `Pattern.compile` of an invalid pattern throws its `PatternSyntaxException` at once.
// (The other POSIX classes, which JavaScript's translator lacks: regex_beyond_javascript.)
import java.util.regex.{Pattern, PatternSyntaxException}

object Main:
  def t(p: String, s: String): Boolean = Pattern.compile(p).matcher(s).matches()
  def main(args: Array[String]): Unit =
    println(s"${t("\\p{Lower}", "é")} ${t("\\p{Lower}", "a")} ${t("\\p{Lower}", "A")}")
    println(s"${t("(?i)\\p{Lower}", "A")} ${t("(?i)\\p{Upper}", "b")}")
    println(s"${t("\\p{Upper}", "\u00c9")} ${t("\\p{Upper}", "Q")} ${t("\\p{Alpha}", "\u00e9")} ${t("(?i)\\p{Alpha}", "Q")} ${t("\\P{Alpha}", "\u00e9")}")
    println(s"${t("[\\p{Lower}\\d]+", "a1é")} ${t("[^\\p{Lower}]", "é")} ${t("[\\p{Alpha}_]+", "ab_Z")}")
    println(s"${t("\\p{Ll}", "é")} ${t("\\p{L}", "é")} ${t("\\p{Lu}", "É")}")
    try
      Pattern.compile("[")
      println("compiled")
    catch case e: PatternSyntaxException => println(e.getClass.getName + " " + e.getPattern)
    try Pattern.compile("a)") catch case e: IllegalArgumentException => println(e.isInstanceOf[PatternSyntaxException])
    println(Pattern.compile("(a)(b)").matcher("ab").matches())
