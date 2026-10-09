//> using platform jvm
// `UNICODE_CHARACTER_CLASS` (`(?U)`) brings `UNICODE_CASE` with it, set and cleared together
// (the JDK's `Pattern.addFlag`, `subFlag`), and under `CASE_INSENSITIVE` a Unicode lowercase,
// uppercase or titlecase class takes all three (`CharPredicates.getPosixPredicate`,
// `getUnicodePredicate`, `forProperty`), scoped flags included. JavaScript's translator has no
// `(?U)` (COMPATIBILITY).
import java.util.regex.Pattern

object Main:
  def t(p: String, s: String): Boolean = Pattern.compile(p).matcher(s).matches()
  def main(args: Array[String]): Unit =
    println(s"${t("(?U)(?i)\\p{Lower}", "A")} ${t("(?iU)é", "É")} ${t("(?U)é", "É")}")
    println(s"${t("(?i)\\p{IsLower}", "Q")} ${t("(?i)\\p{Ll}", "\u01c5")} ${t("\\p{Lt}", "\u01c5")} ${t("\\p{Lu}", "\u01c5")}")
    println(s"${t("(?i:\\p{IsUpper})", "é")} ${t("(?iU:é)", "É")} ${t("(?iU)(?-U)é", "É")} ${t("(?iu)(?-U)é", "É")}")
    println(s"${t("(?i)\\p{Lower}", "é")} ${t("(?U)\\p{Lower}", "é")} ${t("(?U)\\p{Lower}", "É")}")
    val flags = 2 | 256 // CASE_INSENSITIVE | UNICODE_CHARACTER_CLASS
    println(s"${Pattern.compile("\\p{Upper}", flags).matcher("ß").matches()} ${Pattern.compile("ä", flags).matcher("Ä").matches()}")
