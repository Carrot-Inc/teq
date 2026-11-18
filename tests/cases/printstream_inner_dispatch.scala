// An outer java.io.PrintStream over an INNER PrintStream subclass that overrides every member: which of the inner's members each of
// the outer's calls reaches, in what order, and what the outer's checkError answers. Each call on a fresh pair; autoFlush on and off;
// the inner's checkError answering false and true; the outer closed first. The expected lines are scalac's on the JDK.
import java.io.*
class Log extends OutputStream:
  val t = new java.lang.StringBuilder
  def write(b: Int): Unit = t.append("raw(" + b + ");")
  override def write(b: Array[Byte], off: Int, len: Int): Unit = t.append("raw[" + new String(b, off, len, "ISO-8859-1").replace("\n", "\\n") + "];")
  override def flush(): Unit = t.append("raw.flush;")
  override def close(): Unit = t.append("raw.close;")
class Inner(o: Log, answer: Boolean) extends PrintStream(o, false):
  private def l(s: String): Unit = o.t.append(s + ";")
  override def write(b: Int): Unit = { l("in.write(I)"); super.write(b) }
  override def write(b: Array[Byte], off: Int, len: Int): Unit = { l("in.write([B,I,I)"); super.write(b, off, len) }
  override def write(b: Array[Byte]): Unit = { l("in.write([B)"); super.write(b) }
  override def flush(): Unit = { l("in.flush"); super.flush() }
  override def close(): Unit = { l("in.close"); super.close() }
  override def checkError(): Boolean = { l("in.checkError"); answer }
  override def print(s: String): Unit = { l("in.print(String)"); super.print(s) }
  override def print(x: Any): Unit = { l("in.print(Object)"); super.print(x) }
  override def print(c: Char): Unit = { l("in.print(C)"); super.print(c) }
  override def print(i: Int): Unit = { l("in.print(I)"); super.print(i) }
  override def println(): Unit = { l("in.println()"); super.println() }
  override def println(s: String): Unit = { l("in.println(String)"); super.println(s) }
  override def println(x: Any): Unit = { l("in.println(Object)"); super.println(x) }
  override def append(c: Char): PrintStream = { l("in.append(C)"); super.append(c) }
  override def append(s: CharSequence): PrintStream = { l("in.append(CS)"); super.append(s) }
object Probe:
  val calls: List[(String, PrintStream => Any)] = List(
    "print(String)" -> (_.print("ab")), "print(Int)" -> (_.print(7)), "print(Char)" -> (_.print('c')), "print(Object)" -> (_.print(List(1))),
    "println()" -> (_.println()), "println(String)" -> (_.println("ab")), "println(Int)" -> (_.println(7)), "println(Object)" -> (_.println(List(1))),
    "write(Int)" -> (_.write(65)), "write(bytes,off,len)" -> (_.write(Array[Byte](66, 67), 0, 2)), "write(bytes)" -> (_.write(Array[Byte](68))),
    "append(Char)" -> (_.append('e')), "append(CharSequence)" -> (_.append("fg")), "flush()" -> (_.flush()), "close()" -> (_.close()),
    "checkError()" -> (p => "=" + p.checkError()), "printf" -> (_.printf("%d|%s", Integer.valueOf(1), "x")),
    "close(); checkError()" -> (p => { p.close(); "=" + p.checkError() }),
    "close(); print(String)" -> (p => { p.close(); p.print("z") }))
  def main(args: Array[String]): Unit =
    for (name, call) <- calls; af <- List(false, true); answer <- List(false, true) do
      val log = new Log
      val inner = new Inner(log, answer)
      val outer = new PrintStream(inner, af)
      log.t.setLength(0)
      val r = try call(outer) match { case s: String => " " + s; case _ => "" } catch { case e: Throwable => " THROWS " + e.getClass.getName }
      println(name + " | autoFlush " + af + " | inner answers " + answer + " | " + log.t + r)
