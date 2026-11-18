// java.io.PrintStream by what a subclass and the stream under it SEE: `TraceOut` logs every call that reaches
// the stream under the print stream; `Sub` overrides every overridable member but `printf` and `format` (their
// own case: printstream_format_calls), logs its name and calls the class's own. Each call is made on a fresh
// pair, a plain PrintStream and the subclass, with and without autoFlush: which overridable members a call
// reaches and in what order, what is written to the stream under it and when that stream is flushed. The
// expected lines are scalac's on the JDK.
import java.io.*
class TraceOut extends OutputStream:
  val t = new java.lang.StringBuilder
  def write(b: Int): Unit = t.append("out.write(" + b + ");")
  override def write(b: Array[Byte], off: Int, len: Int): Unit = t.append("out.write[" + new String(b, off, len, "ISO-8859-1").replace("\n", "\\n") + "];")
  override def flush(): Unit = t.append("out.flush;")
  override def close(): Unit = t.append("out.close;")
class Sub(o: TraceOut, af: Boolean) extends PrintStream(o, af):
  private def l(s: String): Unit = o.t.append(s + ";")
  override def print(b: Boolean): Unit = { l("print(Z)"); super.print(b) }
  override def print(c: Char): Unit = { l("print(C)"); super.print(c) }
  override def print(i: Int): Unit = { l("print(I)"); super.print(i) }
  override def print(x: Long): Unit = { l("print(J)"); super.print(x) }
  override def print(f: Float): Unit = { l("print(F)"); super.print(f) }
  override def print(d: Double): Unit = { l("print(D)"); super.print(d) }
  override def print(s: Array[Char]): Unit = { l("print([C)"); super.print(s) }
  override def print(s: String): Unit = { l("print(String)"); super.print(s) }
  override def print(x: Any): Unit = { l("print(Object)"); super.print(x) }
  override def println(): Unit = { l("println()"); super.println() }
  override def println(b: Boolean): Unit = { l("println(Z)"); super.println(b) }
  override def println(c: Char): Unit = { l("println(C)"); super.println(c) }
  override def println(i: Int): Unit = { l("println(I)"); super.println(i) }
  override def println(x: Long): Unit = { l("println(J)"); super.println(x) }
  override def println(f: Float): Unit = { l("println(F)"); super.println(f) }
  override def println(d: Double): Unit = { l("println(D)"); super.println(d) }
  override def println(s: Array[Char]): Unit = { l("println([C)"); super.println(s) }
  override def println(s: String): Unit = { l("println(String)"); super.println(s) }
  override def println(x: Any): Unit = { l("println(Object)"); super.println(x) }
  override def append(c: CharSequence): PrintStream = { l("append(CharSequence)"); super.append(c) }
  override def append(c: CharSequence, s: Int, e: Int): PrintStream = { l("append(CharSequence,I,I)"); super.append(c, s, e) }
  override def append(c: Char): PrintStream = { l("append(C)"); super.append(c) }
  override def write(b: Int): Unit = { l("write(I)"); super.write(b) }
  override def write(b: Array[Byte], off: Int, len: Int): Unit = { l("write([B,I,I)"); super.write(b, off, len) }
  override def write(b: Array[Byte]): Unit = { l("write([B)"); super.write(b) }
  override def writeBytes(b: Array[Byte]): Unit = { l("writeBytes"); super.writeBytes(b) }
  override def flush(): Unit = { l("flush"); super.flush() }
  override def close(): Unit = { l("close"); super.close() }
  override def checkError(): Boolean = { l("checkError"); super.checkError() }
  override protected def setError(): Unit = { l("setError"); super.setError() }
  override protected def clearError(): Unit = { l("clearError"); super.clearError() }
object Trace:
  val members: List[(String, PrintStream => Any)] = List(
    "print(true)" -> (_.print(true)), "print('c')" -> (_.print('c')), "print(1)" -> (_.print(1)), "print(2L)" -> (_.print(2L)),
    "print(1.5f)" -> (_.print(1.5f)), "print(2.5)" -> (_.print(2.5)), "print(Array('x','y'))" -> (_.print(Array('x', 'y'))),
    "print(\"s\")" -> (_.print("s")), "print(null: String)" -> (_.print(null: String)), "print(Some(1): Any)" -> (_.print(Some(1): Any)),
    "print(null: Any)" -> (_.print(null: Any)), "print(\"a\\nb\")" -> (_.print("a\nb")), "print('\\n')" -> (_.print('\n')),
    "println()" -> (_.println()), "println(false)" -> (_.println(false)), "println('d')" -> (_.println('d')), "println(3)" -> (_.println(3)),
    "println(4L)" -> (_.println(4L)), "println(3.5f)" -> (_.println(3.5f)), "println(4.5)" -> (_.println(4.5)),
    "println(Array('z'))" -> (_.println(Array('z'))), "println(\"t\")" -> (_.println("t")), "println(List(1,2): Any)" -> (_.println(List(1, 2): Any)),
    "println(null: String)" -> (_.println(null: String)), "println(null: Any)" -> (_.println(null: Any)),
    "printf(\"%d-%s\", 7, \"f\")" -> (_.printf("%d-%s", Integer.valueOf(7), "f")), "format(\"%s+%s%n\", \"a\", \"b\")" -> (_.format("%s+%s%n", "a", "b")),
    "append(\"app\")" -> (_.append("app")), "append('!')" -> (_.append('!')), "append(\"0123456\", 2, 4)" -> (_.append("0123456", 2, 4)),
    "append(null: CharSequence)" -> (_.append(null: CharSequence)),
    "write(65)" -> (_.write(65)), "write(10)" -> (_.write(10)), "write(Array(66,67))" -> (_.write(Array[Byte](66, 67))),
    "write(Array(68,69,70), 1, 2)" -> (_.write(Array[Byte](68, 69, 70), 1, 2)), "writeBytes(Array(71))" -> (_.writeBytes(Array[Byte](71))),
    "flush()" -> (_.flush()), "checkError()" -> (p => "=" + p.checkError()), "close()" -> (_.close()),
    "close(); print(\"s\"); checkError()" -> (p => { p.close(); p.print("s"); "=" + p.checkError() }),
    "close(); write(65); flush(); checkError()" -> (p => { p.close(); p.write(65); p.flush(); "=" + p.checkError() }),
    "close(); close()" -> (p => { p.close(); p.close() }),
    "print(\"\")" -> (_.print("")), "println(\"\")" -> (_.println("")), "write(Array())" -> (_.write(new Array[Byte](0))),
    "append(\"\")" -> (_.append("")), "format(\"\")" -> (_.format("")), "format(\"plain\")" -> (_.format("plain")),
    "format(\"%s\", null)" -> (_.format("%s", null: Object)), "print(1); print(2)" -> (p => { p.print(1); p.print(2) }),
    "println(1); println(2)" -> (p => { p.println(1); p.println(2) }),
    "close(); println(\"x\"); checkError()" -> (p => { p.close(); p.println("x"); "=" + p.checkError() }),
    "close(); append('c'); printf(\"%d\", 1)" -> (p => { p.close(); p.append('c'); p.printf("%d", Integer.valueOf(1)); "=" + p.checkError() }))
  def main(args: Array[String]): Unit =
    for (name, call) <- members; af <- List(true, false); sub <- List(false, true) do
      val o = new TraceOut
      val p: PrintStream = if sub then new Sub(o, af) else new PrintStream(o, af)
      val r = try call(p) match { case s: String => s; case _ => "" } catch { case e: Throwable => "THROWS " + e.getClass.getName }
      println(name + " | " + (if sub then "subclass" else "plain") + " | " + (if af then "autoFlush" else "noFlush") + " | " + o.t + r)
