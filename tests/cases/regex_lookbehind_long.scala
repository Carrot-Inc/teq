// A lookbehind far into a long text, as a script's scan of a document meets it: the match after
// 100,000 characters is found, each position's lookbehind tried from the starts its pattern can
// span alone (a lookbehind tried from every earlier start ran out of the engine's steps).
object Main:
  def main(args: Array[String]): Unit =
    val text = "a" * 100000 + "]x`b`" + "a" * 1000 + "`c`"
    val m = java.util.regex.Pattern.compile("(?<!\\]x)`([a-z])`").matcher(text)
    var found = List.empty[String]
    while m.find() do found = found :+ s"${m.group(1)}@${m.start()}"
    println(found.mkString(" "))
    println("(?<=a{2,3})b".r.findFirstMatchIn("x" * 50000 + "aab").map(_.start))
    println("(?<!a|bc)d".r.findAllMatchIn("ad bcd cd" * 3000).length)
