// The outer stream's checkError when its OWN error is set while it is open (the inner stream's write or flush threw an IOException),
// over an inner PrintStream subclass whose checkError answers false or true, and over a plain inner stream; and after a close: over a
// print stream an open stream answers that stream's checkError, its own error not added. The expected lines are scalac's on the JDK.
import java.io.*
class Log extends OutputStream:
  val t = new java.lang.StringBuilder
  def write(b: Int): Unit = t.append("raw(" + b + ");")
  override def write(b: Array[Byte], off: Int, len: Int): Unit = t.append("raw[" + (len) + "];")
  override def flush(): Unit = t.append("raw.flush;")
class Inner(o: Log, answer: Boolean, throwsWrite: Boolean, throwsFlush: Boolean) extends PrintStream(o, false):
  private def l(s: String): Unit = o.t.append(s + ";")
  override def write(b: Int): Unit = { l("in.write(I)"); if throwsWrite then throw new IOException("w") else super.write(b) }
  override def write(b: Array[Byte], off: Int, len: Int): Unit = { l("in.write([B,I,I)"); if throwsWrite then throw new IOException("w") else super.write(b, off, len) }
  override def flush(): Unit = { l("in.flush"); if throwsFlush then throw new IOException("f") else super.flush() }
  override def checkError(): Boolean = { l("in.checkError"); answer }
object Probe:
  def main(args: Array[String]): Unit =
    for answer <- List(false, true); (tw, tf) <- List((true, false), (false, true), (false, false)); af <- List(false, true) do
      val log = new Log
      val outer = new PrintStream(new Inner(log, answer, tw, tf), af)
      val r1 = try { outer.print("x\n"); "printed" } catch { case e: Throwable => "THROWS " + e.getClass.getSimpleName }
      val c1 = outer.checkError()
      val r2 = try { outer.write(65); "wrote" } catch { case e: Throwable => "THROWS " + e.getClass.getSimpleName }
      val c2 = outer.checkError()
      outer.close()
      val c3 = outer.checkError()
      outer.print("y")
      val c4 = outer.checkError()
      println(s"inner answers $answer, inner write throws $tw, inner flush throws $tf, autoFlush $af | $r1 check=$c1 | $r2 check=$c2 | closed check=$c3 | print after close check=$c4 | ${log.t}")
    val plainLog = new Log
    val plain = new PrintStream(new PrintStream(plainLog), false)
    plain.print("p"); val p1 = plain.checkError(); plain.close(); val p2 = plain.checkError(); plain.print("q"); val p3 = plain.checkError()
    println(s"over a plain print stream | check=$p1 | closed check=$p2 | print after close check=$p3 | ${plainLog.t}")
