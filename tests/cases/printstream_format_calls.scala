// A subclass of java.io.PrintStream that overrides `printf` and `format`: a call reaches the subclass's member,
// `printf` goes to `format`, and the formatter gives a fixed run of the format to `append(format, start, end)`
// and a specifier's text to `append(text)`, each in its turn, on to `print(String)`. The expected lines are
// scalac's on the JDK. On the JVM teq refuses the class (tests/errors/printstream_printf_override_jvm): the JDK's
// `printf` takes an array where the std's takes a sequence, and the bridge is not written yet.
import java.io.*
class TraceOut extends OutputStream:
  val t = new java.lang.StringBuilder
  def write(b: Int): Unit = t.append("out.write(" + b + ");")
  override def write(b: Array[Byte], off: Int, len: Int): Unit = t.append("out.write[" + new String(b, off, len, "ISO-8859-1").replace("\n", "\\n") + "];")
  override def flush(): Unit = t.append("out.flush;")
class Sub(o: TraceOut, af: Boolean) extends PrintStream(o, af):
  private def l(s: String): Unit = o.t.append(s + ";")
  override def print(s: String): Unit = { l("print(String)"); super.print(s) }
  override def printf(f: String, a: Any*): PrintStream = { l("printf"); super.printf(f, a*) }
  override def format(f: String, a: Any*): PrintStream = { l("format"); super.format(f, a*) }
  override def append(c: CharSequence): PrintStream = { l("append(CharSequence)"); super.append(c) }
  override def append(c: CharSequence, s: Int, e: Int): PrintStream = { l("append(CharSequence,I,I)"); super.append(c, s, e) }
object Trace:
  val calls: List[(String, PrintStream => Any)] = List(
    "printf(\"%d-%s\", 7, \"f\")" -> (_.printf("%d-%s", Integer.valueOf(7), "f")),
    "format(\"%s+%s%n\", \"a\", \"b\")" -> (_.format("%s+%s%n", "a", "b")),
    "format(\"\")" -> (_.format("")), "format(\"plain\")" -> (_.format("plain")),
    "format(\"%s\", null)" -> (_.format("%s", null: Object)),
    "format(\"%2$s %1$s %<s\", \"a\", \"b\")" -> (_.format("%2$s %1$s %<s", "a", "b")),
    "format(\"%5d|%-5s|%05.1f%%\", 42, \"ab\", 2.25)" -> (_.format("%5d|%-5s|%05.1f%%", Integer.valueOf(42), "ab", java.lang.Double.valueOf(2.25))),
    "close(); printf(\"%d\", 1)" -> (p => { p.close(); p.printf("%d", Integer.valueOf(1)); "=" + p.checkError() }))
  def main(args: Array[String]): Unit =
    for (name, call) <- calls; af <- List(true, false); sub <- List(false, true) do
      val o = new TraceOut
      val p: PrintStream = if sub then new Sub(o, af) else new PrintStream(o, af)
      val r = try call(p) match { case s: String => s; case _ => "" } catch { case e: Throwable => "THROWS " + e.getClass.getName }
      println(name + " | " + (if sub then "subclass" else "plain") + " | " + (if af then "autoFlush" else "noFlush") + " | " + o.t + r)
