// Unicode properties under `(?i)`, as JDK 24 reads them: `Lu`, `Ll` and `Lt` (written alone or
// as a `gc=` value) are the three categories together, inside a class and outside one,
// complemented by `\P` or by the class; no other property folds, and the ASCII letters beside
// them fold as `(?i)` folds them, so that a plain `(?i)` never takes the engine's own folding
// (the Kelvin sign is no `k`, the long s no `s`). Under `(?iu)` the engine folds, and the
// combining ypogegrammeni, whose folding is iota's, is left out here.
object Main:
  val cased = List("a", "A", "z", "\u00e9", "\u00c9", "\u0131", "\u0130", "\u01c4", "\u01c5", "\u01c6", "\u017f", "\u212a", "k", "K", "1", "\u03b9", "\u0399", "\u1fbe", "\u00aa", "\u02b0", "\u2170", "\u24b6", "\uD835\uDC00", "\uD801\uDC28", "_")
  def shown(s: String): String =
    val c = s.codePointAt(0)
    if c < 0x21 || c > 0x7e then "U+" + Integer.toHexString(c).toUpperCase else s
  def members(p: String, alphabet: List[String]): String =
    try
      val r = p.r
      alphabet.filter(s => r.matches(s)).map(shown).mkString(" ")
    catch case _: IllegalArgumentException => "rejected"
  def main(args: Array[String]): Unit =
    val plain = List(
      "\\p{Lu}", "(?i)\\p{Lu}", "(?i)\\p{Ll}", "(?i)\\p{Lt}", "(?i)\\P{Lu}", "(?i)[\\p{Lu}]", "(?i)[^\\p{Lu}]",
      "(?i)[^\\P{Ll}]", "(?i)[\\P{Lt}]", "(?i)\\p{gc=Lu}", "(?i)\\p{General_Category=Ll}", "(?i)\\p{L}", "(?i)\\p{Lm}",
      "(?i)\\pL", "(?i)[\\p{Lu}&&[^a-c]]", "(?i)[\\p{Lu}\\d]", "(?i)[^\\p{Lu}\\d]", "(?i)[a\\P{Lu}]",
      "(?i)[k\\p{Lo}]", "(?i)[s-u\\p{Lm}]"
    )
    for p <- plain do println(p + " = " + members(p, cased))
    val folded = List("(?iu)\\p{Lu}", "(?iu)\\P{Lu}", "(?iu)[^\\p{Lt}]", "(?iu)[\\P{Ll}]", "(?iu)[^\\P{Lu}]", "(?iu)\\p{L}", "(?iu)\\P{L}", "(?iu)[\\P{Ll}a]")
    for p <- folded do println(p + " = " + members(p, cased))
    for (p, s) <- List(("(?i)[\\p{Lu}]k", "\u00e9k"), ("(?i)[\\p{Lu}]k", "\u00e9K"), ("(?i)[\\p{Lu}]k", "\u00e9\u212a"), ("(?i)\\p{Lu}s", "\u00e9S"),
        ("(?i)\\p{Lu}s", "\u00e9\u017f"), ("(?iu)\\p{Lu}k", "\u00e9\u212a")) do
      println(p + " on " + s.map(c => if c > '~' then "U+" + Integer.toHexString(c).toUpperCase + " " else c.toString).mkString.trim + " = " + (try s.matches(p).toString catch case _: IllegalArgumentException => "rejected"))
    println("(?i)(\\p{Lu}+)-(\\P{Ll}+)".r.findAllMatchIn("abc-DEF xy-12 \u01c4-\u01c6").map(m => m.group(1) + "/" + m.group(2)).toList)
