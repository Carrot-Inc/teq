// A format the JDK's formatter refuses is refused before anything is written, whatever stands before the bad
// specifier; a missing argument is found when its turn comes, after the pieces before it. A closed stream reads
// nothing: no exception, its error is set. The JDK throws subclasses of IllegalArgumentException that the std
// does not have: the lines name the class they share and the JDK's message. The expected lines are scalac's on
// the JDK.
import java.io.{ByteArrayOutputStream, PrintStream}
object Probe:
  def t(name: String)(body: PrintStream => Any): Unit =
    val b = new ByteArrayOutputStream
    val p = new PrintStream(b, true, "UTF-8")
    val r =
      try { body(p); "ok" }
      catch
        case e: IllegalArgumentException => "IllegalArgumentException: " + e.getMessage
        case e: NullPointerException => "NullPointerException"
    println(name + " -> " + r + " | [" + b.toString("UTF-8") + "] error=" + p.checkError())
  def main(args: Array[String]): Unit =
    val formats = List("%", "% ", "%-", "%5", "%.3", "%5.", "%1$", "%<", "%t", "x%", "%,", "%#", "%-5", "%05", "a%5.2", "%5n", "%-5n", "%.2n", "%5%",
      "%-5%", "%.2%", "%1$5", "%$s", "%0$s", "%2$s", "%tq", "%Tq", "%q", "%5q", "%-q", "%.q", "%1$q", "ab%sc%", "%s%q", "%s% ", "%!", "%5!", "%-!",
      "%--5s", "%++d", "%.5d", "%-d", "%0d", "%b", "%x", "%#x", "%o", "%X", "%B", "%3$s", "%<s", "%s|%s|%s", "%s %<s %s", "%2$s %s %<s %s")
    for f <- formats do t("[" + f + "]")(_.format(f, Integer.valueOf(7), "b"))
    t("null format")(_.format(null: String, "a"))
    t("null printf")(_.printf(null: String, "a"))
    t("no arguments")(_.format("a%s"))
    t("bad after written pieces")(_.format("a%s|%5q", "x", "y"))
    t("too few after written pieces")(_.format("a%s|%s", "x"))
    t("duplicate flag after written pieces")(_.format("a%s|%--5s", "x", "y"))
    t("width on a line end")(_.format("a%s|%5n", "x"))
    t("closed, null format")(p => { p.close(); p.format(null: String, "a") })
    t("closed, null printf")(p => { p.close(); p.printf(null: String, "a") })
    t("closed, bad format")(p => { p.close(); p.format("%q", "a") })
    t("closed, too few")(p => { p.close(); p.format("%s%s", "a") })
    t("closed, a format")(p => { p.close(); p.format("%s", "a") })
