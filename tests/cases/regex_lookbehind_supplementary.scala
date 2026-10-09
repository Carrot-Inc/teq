//> using platform js
// interp-expected: js
// A lookbehind before a supplementary character. The JDK reads a pattern as one over supplementary
// characters only where such a character is written in its text (`Pattern.findSupplementary`), so
// that with the character escaped (`\x{1D11E}`) its lookbehind steps back one UTF-16 unit and finds
// no `x` after the pair, where the character written as itself finds the one at 2. JavaScript's
// engine and the interpreter step back over the whole pair both ways;
// `tests/jvm-expected/` holds the JVM's answers.
object Main:
  def main(args: Array[String]): Unit =
    val g = "𝄞"
    val patterns = List(
      "escaped in an intersection" -> "(?<=[\\x{1D11E}&&[\\x{1D11E}]])x",
      "written in an intersection" -> ("(?<=[" + g + "&&[" + g + "]])x"),
      "escaped in a class" -> "(?<=[\\x{1D11E}])x",
      "written in a class" -> ("(?<=[" + g + "])x"),
      "escaped alone" -> "(?<=\\x{1D11E})x",
      "written alone" -> ("(?<=" + g + ")x")
    )
    for (label, p) <- patterns do println(label + " = " + p.r.findAllMatchIn(g + "x").map(_.start).toList)
