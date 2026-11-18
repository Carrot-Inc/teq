// A held high half of a surrogate pair by what a subclass and the stream under it SEE: at close it is written
// first, through the stream's own `write(bytes, off, len)`; a flush alone writes nothing for it; the low half,
// a byte, a line end after it. On a plain PrintStream and a subclass, with and without autoFlush. The expected
// lines are scalac's on the JDK.
import java.io.*
class TraceOut extends OutputStream:
  val t = new java.lang.StringBuilder
  def write(b: Int): Unit = t.append("out.write(" + b + ");")
  override def write(b: Array[Byte], off: Int, len: Int): Unit = t.append("out.write[" + b.slice(off, off + len).map(_ & 255).mkString(",") + "];")
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
    "print(high); close()" -> (p => { p.print('\uD83D'); p.close() }),
    "print(high); flush(); close()" -> (p => { p.print('\uD83D'); p.flush(); p.close() }),
    "print(high); checkError()" -> (p => { p.print('\uD83D'); "=" + p.checkError() }),
    "print(high); print(low); close()" -> (p => { p.print('\uD83D'); p.print('\uDE00'); p.close() }),
    "print(high); write(65); close()" -> (p => { p.print('\uD83D'); p.write(65); p.close() }),
    "print(high); println(); close()" -> (p => { p.print('\uD83D'); p.println(); p.close() }))
  def main(args: Array[String]): Unit =
    for (name, call) <- members; af <- List(true, false); sub <- List(false, true) do
      val o = new TraceOut
      val p: PrintStream = if sub then new Sub(o, af) else new PrintStream(o, af)
      val r = try call(p) match { case s: String => s; case _ => "" } catch { case e: Throwable => "THROWS " + e.getClass.getName }
      println(name + " | " + (if sub then "subclass" else "plain") + " | " + (if af then "autoFlush" else "noFlush") + " | " + o.t + r)
