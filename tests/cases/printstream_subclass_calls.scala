// What a subclass of java.io.PrintStream sees of a println: of an object, its print of a String and then the line's
// end; the flush at a line's end is the stream's under it, not the print stream's own; a plain stream writes the
// text and the line's end as one.
import java.io.*
class TraceOut extends OutputStream:
  var trace = ""
  def write(b: Int): Unit = trace += "byte;"
  override def write(b: Array[Byte], off: Int, len: Int): Unit = trace += "bytes:" + len + ";"
  override def flush(): Unit = trace += "out.flush;"
  override def close(): Unit = trace += "out.close;"
class TracePrint(o: TraceOut) extends PrintStream(o, true):
  override def print(s: String): Unit = { o.trace += "print.String;"; super.print(s) }
  override def print(s: Any): Unit = { o.trace += "print.Object;"; super.print(s) }
  override def println(): Unit = { o.trace += "println.zero;"; super.println() }
  override def println(s: String): Unit = { o.trace += "println.String;"; super.println(s) }
  override def write(b: Array[Byte], off: Int, len: Int): Unit = { o.trace += "write;"; super.write(b, off, len) }
  override def flush(): Unit = { o.trace += "ps.flush;"; super.flush() }
  override def close(): Unit = { o.trace += "ps.close;"; super.close() }
object Probe:
  def main(args: Array[String]): Unit =
    val o = new TraceOut
    val p = new TracePrint(o)
    p.println("x": Any)
    System.out.println("object=" + o.trace)
    o.trace = ""
    p.println("x")
    System.out.println("string=" + o.trace)
    o.trace = ""
    p.close()
    System.out.println("close=" + o.trace)
    val qout = new TraceOut
    new PrintStream(qout, true).println("x")
    System.out.println("plain=" + qout.trace)
