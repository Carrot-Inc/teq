//> using platform jvm
// What JavaScript's regex translator lacks (COMPATIBILITY: the regular expressions on JavaScript), in
// the interpreter and on the JVM: an atomic group `(?>X)` takes the first way `X` matches and never
// gives it back, as a possessive quantifier does (`X*+` is `(?>X*)`); Java's POSIX classes are of
// ASCII characters alone but under UNICODE_CHARACTER_CLASS.
import java.util.regex.Pattern

object Main:
  def t(p: String, s: String): Boolean = Pattern.compile(p).matcher(s).matches()
  def main(args: Array[String]): Unit =
    println(s"${t("(?>a+)a", "aa")} ${t("(?>a|ab)c", "abc")} ${t("(?>a|ab)c", "ac")}")
    println(s"${t("x(?>y*)y", "xyy")} ${t("x(?>y*)z", "xyyz")}")
    val m = Pattern.compile("(?>(a+))b").matcher("aab")
    println(s"${m.matches()} ${m.group(1)}")
    val f = Pattern.compile("(?>a*)b").matcher("xaab")
    println(s"${f.find()} ${f.start()}")
    println(s"${t("(?>a+|b)+c", "aabc")} ${t("a++a", "aa")}")
    println(s"${t("\\p{Alnum}", "\u0663")} ${t("\\p{Digit}", "\u0663")} ${t("\\p{Punct}", "\u00a1")} ${t("\\p{Punct}", "!")}")
    println(s"${t("\\p{Graph}", "\u00e9")} ${t("\\p{Print}", " ")} ${t("\\p{Cntrl}", "\u0085")} ${t("\\p{Space}", "\u00a0")}")
    println(s"${t("\\p{XDigit}", "f")} ${t("[\\p{Alnum}_]+", "ab_1")} ${t("(?U)\\p{Lower}", "\u00e9")}")
