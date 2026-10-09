//> using platform jvm
// os: unix
// The streams of a child against the JDK: its stdin written and read back, buffered until a flush
// as the JDK's pipe streams are; a child that fills both of its outputs before it reads its stdin,
// drained without threads by reading what each has (`available`) in turn; a child that never ends
// its line; a character whose bytes arrive in two writes, joined by the reader; lines ended by `\n`,
// `\r\n` and `\r`; `read` into a buffer, `readNBytes`, `skip`, `transferTo`; a write after the child
// ended and a read after a close refused. The children are `sh`, `cat` and `printf`.
import java.io.{BufferedReader, ByteArrayOutputStream, InputStreamReader}

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def sh(script: String): Process = new ProcessBuilder("sh", "-c", script).start()

  def main(args: Array[String]): Unit =
    // Written, flushed, read back: a line at a time from `cat`.
    val cat = new ProcessBuilder("cat").start()
    val out = cat.getOutputStream
    val in = new BufferedReader(new InputStreamReader(cat.getInputStream, "UTF-8"))
    out.write("first line\n".getBytes("UTF-8"))
    out.flush()
    show("echoed")(in.readLine())
    out.write("second, é\n".getBytes("UTF-8"))
    out.flush()
    show("echoed")(in.readLine())
    out.close()
    show("after the close")(in.readLine())
    show("cat")(cat.waitFor())
    show("write after the end")({ out.write(1); out.flush(); "written" })
    // Both outputs filled past a pipe's capacity before the child reads its stdin.
    val big = sh("head -c 200000 /dev/zero; head -c 300000 /dev/zero >&2; read line; printf '%s' \"$line\"")
    val (o, e) = (big.getInputStream, big.getErrorStream)
    val got = new ByteArrayOutputStream()
    var outBytes = 0L
    var errBytes = 0L
    val chunk = new Array[Byte](8192)
    while outBytes < 200000 || errBytes < 300000 do
      if o.available() > 0 then outBytes += o.read(chunk, 0, chunk.length)
      else if e.available() > 0 then errBytes += e.read(chunk, 0, chunk.length)
      else Thread.sleep(1)
    big.getOutputStream.write("the answer\n".getBytes)
    big.getOutputStream.close()
    show("both outputs")(s"$outBytes and $errBytes")
    show("then stdin")(new String(o.readAllBytes()))
    show("big")(big.waitFor())
    // No newline at the end: the line is what came before the end.
    val partial = new BufferedReader(new InputStreamReader(sh("printf 'no newline'").getInputStream))
    show("partial")(partial.readLine())
    show("partial end")(partial.readLine())
    // A character in two writes, a pause between.
    val split = new BufferedReader(new InputStreamReader(sh("printf 'h\\303'; sleep 0.2; printf '\\251llo\\n'").getInputStream, "UTF-8"))
    show("split character")(split.readLine())
    // Line ends.
    val ends = new BufferedReader(new InputStreamReader(sh("printf 'a\\nb\\r\\nc\\rd\\r\\r\\ne'").getInputStream))
    var line = ends.readLine()
    val lines = scala.collection.mutable.ListBuffer.empty[String]
    while line != null do
      lines += line
      line = ends.readLine()
    show("lines")(lines.mkString("[", "|", "]"))
    show("lines()")(new BufferedReader(new InputStreamReader(sh("printf 'x\\ny\\n'").getInputStream)).lines().toArray.mkString(","))
    // Bytes.
    val bytes = sh("printf 'abcdefghij'").getInputStream
    val buf = new Array[Byte](4)
    show("read into")(s"${bytes.read(buf, 1, 2)} ${new String(buf, 1, 2)}")
    show("skip")(bytes.skip(2))
    show("readNBytes")(new String(bytes.readNBytes(3)))
    show("rest")(new String(bytes.readAllBytes()))
    show("at the end")(bytes.read())
    show("readNBytes at the end")(bytes.readNBytes(5).length)
    bytes.close()
    show("read after close")(bytes.read())
    show("available after close")(bytes.available())
    val sink = new ByteArrayOutputStream()
    show("transferTo")(s"${sh("printf 'twelve bytes'").getInputStream.transferTo(sink)} $sink")
    // Characters one at a time, and a buffer of them.
    val chars = new InputStreamReader(sh("printf 'añb'").getInputStream)
    show("chars")(List(chars.read(), chars.read(), chars.read(), chars.read()).mkString(","))
    val cbuf = new Array[Char](8)
    val reader = new BufferedReader(new InputStreamReader(sh("printf 'xyz'").getInputStream))
    show("char buffer")(s"${reader.read(cbuf, 0, 8)} ${new String(cbuf, 0, 3)}")
    show("char buffer end")(reader.read(cbuf, 0, 8))
