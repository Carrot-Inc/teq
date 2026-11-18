// The stream's error by its own answers. An IOException of a subclass's own `write(bytes, off, len)` on the text
// path (print, println, printf, append) is the stream's error, not the caller's exception; a byte write the caller
// makes itself passes it on. A closed stream answers its own state to `checkError`, without asking the print
// stream under it. After `close` a null char array, a null format and a bad one throw nothing. The expected
// lines are scalac's on the JDK.
import java.io.*
class Log extends OutputStream:
  val sb = new java.lang.StringBuilder
  def write(b: Int): Unit = sb.append("w(" + b + ");")
  override def write(b: Array[Byte], off: Int, len: Int): Unit = sb.append("w[" + b.slice(off, off + len).mkString(",") + "];")
  override def flush(): Unit = sb.append("f;")
  override def close(): Unit = sb.append("c;")
class Sub(o: OutputStream, af: Boolean) extends PrintStream(o, af):
  override def write(b: Array[Byte], off: Int, len: Int): Unit = throw new IOException("subwrite")
object Probe:
  def chk(label: String)(body: => Any): Unit =
    try println(label + " = " + body)
    catch
      case t: IOException => println(label + " ! IOException: " + t.getMessage)
      case t: NullPointerException => println(label + " ! NullPointerException")
  def main(args: Array[String]): Unit =
    for af <- List(true, false) do
      def on(label: String)(call: PrintStream => Any): Unit =
        val l = new Log
        val s = new Sub(l, af)
        chk("a throwing write, autoFlush " + af + ", " + label)({ call(s); l.sb.toString + s.checkError() })
      on("print")(_.print("x"))
      on("println")(_.println("x"))
      on("println()")(_.println())
      on("printf")(_.printf("a%sb", "x"))
      on("append")(_.append('c'))
      on("append text")(_.append("cd"))
      on("print chars")(_.print(Array('a', 'b')))
      on("write bytes")(_.write(Array[Byte](1)))
      on("write bytes, off, len")(_.write(Array[Byte](1, 2), 0, 2))
    val l4 = new Log
    val in4 = new PrintStream(l4, false)
    val out4 = new PrintStream(in4, true)
    chk("over a print stream, println")({ out4.println("x"); l4.sb.toString })
    chk("over a print stream, close")({ out4.close(); l4.sb.toString })
    chk("the one under it after the close")({ in4.print("q"); l4.sb.toString + in4.checkError() + out4.checkError() })
    val l5 = new Log
    val in5 = new PrintStream(l5, false)
    val out5 = new PrintStream(in5, false)
    chk("the one under it closed first")({ in5.close(); out5.print("x"); l5.sb.toString + out5.checkError() })
    val l6 = new Log
    val p6 = new PrintStream(l6, true)
    chk("closed, then members")({ p6.close(); p6.append("a"); p6.append('c'); p6.write(Array[Byte](1)); p6.printf("%d", 1); p6.println(); p6.flush(); l6.sb.toString })
    chk("closed, print of a null char array")({ p6.print(null: Array[Char]); "none" })
    chk("closed, println of a null char array")({ p6.println(null: Array[Char]); "none" })
    chk("closed, a null format")({ p6.printf(null: String, "a"); "none" })
    chk("closed, a bad format")({ p6.format("%q", "a"); "none" })
    chk("closed, its error")(p6.checkError())
    val l7 = new Log
    val p7 = new PrintStream(l7, true)
    chk("open, print of a null char array")({ p7.print(null: Array[Char]); "none" })
    chk("open, its error")(p7.checkError())
