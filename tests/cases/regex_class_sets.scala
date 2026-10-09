// Character classes as sets, read as JDK 24's `Pattern.clazz` and `range` read them: nested
// classes join the union, `&&` intersects what precedes with the operand after it (bare or
// bracketed, several in a row), the characters below U+0100 named alone form one set that every
// operand of the class shares as it is at the class's end, an empty operand repeats the last
// element, a leading `]` is itself and a hyphen before `[` or `]` too. On JavaScript a set that no
// single class of the engine's holds is written with lookaheads, in the same Unicode mode as the
// rest of the pattern.
object Main:
  val alphabet: List[String] =
    (0x20 to 0x7e).map(_.toChar.toString).toList ++ List("\u00e9", "\u00c9", "\u0131", "\u017f", "\u212a", "\u03a9", "\uD834\uDD1E", "\n", "\t", "\u00a0")
  def shown(s: String): String =
    val c = s.codePointAt(0)
    if c < 0x21 || c > 0x7e then "U+" + Integer.toHexString(c).toUpperCase else s
  def members(p: String): String =
    try
      val r = p.r
      alphabet.filter(s => r.matches(s)).map(shown).mkString(" ")
    catch case _: IllegalArgumentException => "rejected"
  def main(args: Array[String]): Unit =
    val patterns = List(
      "[a[bc]d]", "[a&&[b]&c]", "[a-z&&]", "[a-cx-z&&]", "[a-cx&&]", "[a-[bc]]", "[a-z&&[^aeiou]]", "[ab&&bc]",
      "[&&a]", "[&&]", "[a&&]", "[]a]", "[^]a]", "[a-]", "[a-z&&[aeiou]&&[^e]]", "[a[^b]]", "[^a[^bc]]",
      "[\\w&&[^\\d]]", "[a&&b&&c]", "[abc&&b&&bc]", "[a-c[x]&&[b-x]]", "[^a-z&&[^aeiou]]", "[a-z&&b-y&&c-x]",
      "[[a-c]&&[b-d]&&[c-e]]", "[a&&&b]", "[&a]", "[a&b]", "[^a[b]]", "[a-c[x-z]]", "[^[a-c][x-z]]",
      "[\\p{L}&&[^a-z]]", "[\\P{L}&&[^\\d]]", "[\\s&&[^\\n]]", "[\\S&&[a-c]]", "[\\h[a]]", "[^\\H&&[^\\t]]",
      "[\\x{1D11E}[a]]", "[^\\x{1D11E}&&[^a-y]]", "[a-z&&[^\\x{1D11E}]]", "[\\p{Lu}&&[A-F]]", "[^\\p{L}[a]]",
      "(?i)[a-z&&[^aeiou]]", "(?i)[A-Z&&[^AEIOU]]", "(?i)[k&&[^K]]", "(?iu)[a-e&&[^\\u00c9]]", "(?iu)[\\u00e0-\\u00ff&&[e\\u00e9]]",
      "[a-z&&", "[a&&[b]", "[z-a]", "[\\b]", "[a-\\d]", "[\\d-z]"
    )
    for p <- patterns do println(p + " = " + members(p))
    // The set in its pattern: quantified, beside groups and back references, behind a lookbehind.
    println("[a-z&&[^aeiou]]+".r.findAllIn("strength and rhythm").toList)
    println("([a-z&&[^aeiou]])\\1".r.findAllIn("bookkeeper").toList)
    println("(?<=[a-z&&[^aeiou]])[aeiou]".r.findAllMatchIn("banana tea").map(_.start).toList)
    println("(?<![a[bc]])x".r.findAllMatchIn("ax bx dx x").map(_.start).toList)
    println("[\\p{Lu}&&[^A-Z]][a-z&&[^x]]*".r.findAllIn("\u00c9lan Zed \u00c5ke").toList)
    println("\\P{L}[a[b]]".r.findAllIn("1a 2b 3c").toList)
    println("[a-c[x]&&[^b]]{2}".r.findAllIn("acxbxa").toList)
    // Empty operands nested twelve deep, each repeating the class before it.
    val deep = (1 to 12).foldLeft("[a&&a]")((p, _) => "[" + p + "&&]")
    println(deep.length + " " + List("a", "b").map(s => s.matches(deep)))
    val beside = (1 to 12).foldLeft("[a]")((p, _) => "[x" + p + "&&]")
    println(beside.length + " " + List("a", "x", "b").map(s => s.matches(beside)))
    // A set the JDK refuses, split over the empty input: refused all the same.
    println(try "[a&&[&&]]".r.split("").toList.toString catch case _: IllegalArgumentException => "rejected")
