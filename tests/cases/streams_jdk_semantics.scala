//> using platform jvm
// os: unix
// Streams as the JDK's classes behave: a file's output stream writes through, before a flush or a
// close; `FileInputStream.skip` seeks and may pass the end, or go back; `Files.newInputStream`'s
// skip moves its channel, the end at most; a child's stdout skips what its buffer holds, then reads
// past the rest; a closed stream refuses a read or a write as its class does; `Files.newBufferedReader`
// reports malformed UTF-8 where `InputStreamReader` replaces it; what a child wrote before it ended
// is read after its end.
import java.io.*
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path}

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName + ": " + e.getMessage
    println(s"$name: $r")

  def main(args: Array[String]): Unit =
    val dir = Files.createTempDirectory("streams")
    val p = dir.resolve("data")
    val out = new FileOutputStream(p.toFile)
    out.write('A')
    show("written before the close")(Files.size(p))
    out.write("BC".getBytes)
    show("and more")(Files.size(p))
    out.close()
    val nout = Files.newOutputStream(p)
    nout.write("abcdef".getBytes)
    show("Files' stream writes through")(Files.size(p))
    nout.close()

    val in = new FileInputStream(p.toFile)
    show("read")(in.read().toChar)
    show("skip 2")(in.skip(2))
    show("then read")(in.read().toChar)
    show("skip back 2")(in.skip(-2))
    show("then read again")(in.read().toChar)
    show("available")(in.available())
    show("skip past the end")(in.skip(100))
    show("read past the end")(in.read())
    in.close()
    show("read when closed")(in.read())

    val chan = Files.newInputStream(p)
    show("channel read")(chan.read().toChar)
    show("channel skip 2")(chan.skip(2))
    show("channel read")(chan.read().toChar)
    show("channel skip back 10")(chan.skip(-10))
    show("channel read")(chan.read().toChar)
    show("channel skip past the end")(chan.skip(100))
    show("channel skip at the end")(chan.skip(5))
    chan.close()
    show("channel read when closed")(chan.read())
    val fo = new FileOutputStream(p.toFile, true)
    fo.close()
    show("file write when closed")(fo.write(1))
    val no = Files.newOutputStream(p, java.nio.file.StandardOpenOption.APPEND)
    no.close()
    show("channel write when closed")(no.write(1))

    val child = new ProcessBuilder("printf", "abcdefg").start()
    child.waitFor()
    val cin = child.getInputStream
    show("child read")(cin.read().toChar)
    show("child skip, buffered")(cin.skip(100))
    show("child skip at the end")(cin.skip(100))
    show("child read at the end")(cin.read())

    val late = new ProcessBuilder("printf", "kept").start()
    show("child status")(late.waitFor())
    Thread.sleep(50)
    show("read after its end")(new String(late.getInputStream.readAllBytes(), UTF_8))
    show("status again")(late.exitValue())

    Files.write(p, Array(0x61, 0xc3, 0x28, 0x62).map(_.toByte))
    show("Files reader")({
      val r = Files.newBufferedReader(p)
      try r.readLine() finally r.close()
    })
    show("stream reader")({
      val r = new BufferedReader(new InputStreamReader(Files.newInputStream(p), UTF_8))
      try r.readLine().map(_.toInt).mkString(" ") finally r.close()
    })
    Files.write(p, Array(0x61, 0xe2, 0x82).map(_.toByte))
    show("Files reader, a cut end")({
      val r = Files.newBufferedReader(p)
      try r.readLine() finally r.close()
    })
    Files.write(p, Array(0x61, 0xed, 0xa0, 0x80, 0x62).map(_.toByte))
    show("Files reader, a surrogate")({
      val r = Files.newBufferedReader(p)
      try r.readLine() finally r.close()
    })
    show("readAllLines")(Files.readAllLines(p))
    Files.delete(p)
    Files.delete(dir)
