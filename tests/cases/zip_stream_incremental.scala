//> using platform jvm
// The JDK's streaming readers read as they go: `ZipInputStream.getNextEntry` reads one local header
// and no more, `read` takes the entry's data from the source, and a data descriptor's sizes and
// CRC-32 come into the entry at its end (-1 before); a source that gives one byte a read is read
// the same; `GZIPInputStream` reads its header when it is made and its data when it is read, a
// member after another read on, a bad trailer refused; `Inflater` takes its input in pieces and
// tells what it left past the stream's end.
import java.io.*
import java.util.zip.*

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName + ": " + e.getMessage
    println(s"$name: $r")

  // A source over the bytes that fails a read past `limit`, counting what was asked of it.
  final class Guarded(bytes: Array[Byte], limit: Int, step: Int) extends InputStream:
    var at = 0
    def read(): Int =
      if at >= limit then throw new IOException("read past " + limit)
      if at >= bytes.length then -1
      else
        at += 1
        bytes(at - 1) & 0xff
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      if len == 0 then 0
      else if at >= bytes.length then -1
      else
        val n = Math.min(Math.min(len, step), bytes.length - at)
        if at + n > limit then throw new IOException("read past " + limit)
        System.arraycopy(bytes, at, b, off, n)
        at += n
        n

  def main(args: Array[String]): Unit =
    val buf = new ByteArrayOutputStream()
    val z = new ZipOutputStream(buf)
    val text = ("a line of the first entry\n" * 400).getBytes
    for (name, data, stored) <- List(("first", text, false), ("second", "valid".getBytes, false), ("third", "plain".getBytes, true)) do
      val e = new ZipEntry(name)
      e.setTime(0L)
      if stored then
        e.setMethod(ZipEntry.STORED)
        e.setSize(data.length.toLong)
        val c = new CRC32()
        c.update(data)
        e.setCrc(c.getValue())
      z.putNextEntry(e)
      z.write(data)
      z.closeEntry()
    z.close()
    val bytes = buf.toByteArray
    val header = 30 + 5 + 9
    val zin = new ZipInputStream(new Guarded(bytes, header, Int.MaxValue))
    show("first entry from its header alone")(zin.getNextEntry().getName)

    val source = new Guarded(bytes, Int.MaxValue, 1)
    val in = new ZipInputStream(source)
    val first = in.getNextEntry()
    show("taken for the header")(source.at)
    show("size before reading")(s"${first.getSize} ${first.getCompressedSize} ${first.getCrc}")
    val data = in.readAllBytes()
    show("read")(s"${data.length} ${java.util.Arrays.equals(data, text)}")
    show("size after reading")(s"${first.getSize} ${first.getCompressedSize} ${first.getCrc}")
    val second = in.getNextEntry()
    show("second")(s"${second.getName} ${new String(in.readAllBytes())} ${second.getSize}")
    val third = in.getNextEntry()
    show("third, stored")(s"${third.getName} ${third.getMethod} ${third.getSize} ${new String(in.readAllBytes())}")
    show("no more")(String.valueOf(in.getNextEntry()))
    in.close()

    val skipping = new ZipInputStream(new ByteArrayInputStream(bytes))
    skipping.getNextEntry()
    show("skip in an entry")(skipping.skip(30))
    show("next without reading the rest")(skipping.getNextEntry().getName)
    skipping.close()

    val gz = new ByteArrayOutputStream()
    for part <- List("one member, ", "and another") do
      val g = new GZIPOutputStream(gz)
      g.write(part.getBytes)
      g.finish()
    val gzBytes = gz.toByteArray
    val gzGuard = new Guarded(gzBytes, 10, Int.MaxValue)
    show("gzip header alone")({ new GZIPInputStream(gzGuard); gzGuard.at })
    show("gzip members")(new String(new GZIPInputStream(new Guarded(gzBytes, Int.MaxValue, 3)).readAllBytes()))
    val bad = gzBytes.clone()
    bad(gzBytes.length - 1) = (bad(gzBytes.length - 1) + 1).toByte
    show("gzip trailer")(new String(new GZIPInputStream(new ByteArrayInputStream(bad)).readAllBytes()))
    show("not gzip")(new GZIPInputStream(new ByteArrayInputStream("plain".getBytes)))
    show("gzip cut short")(new GZIPInputStream(new ByteArrayInputStream(java.util.Arrays.copyOf(gzBytes, 20))).readAllBytes().length)

    val packed = new ByteArrayOutputStream()
    val d = new DeflaterOutputStream(packed)
    d.write(text)
    d.close()
    val zlib = packed.toByteArray ++ "rest".getBytes
    val inf = new Inflater()
    val out = new Array[Byte](100000)
    var produced = 0
    var at = 0
    while !inf.finished() do
      if inf.needsInput() then
        val n = Math.min(37, zlib.length - at)
        inf.setInput(zlib, at, n)
        at += n
      produced += inf.inflate(out, produced, out.length - produced)
    show("inflated in pieces")(s"$produced ${java.util.Arrays.equals(java.util.Arrays.copyOf(out, produced), text)} remaining ${inf.getRemaining()} read ${inf.getBytesRead()} written ${inf.getBytesWritten()}")
    inf.end()
    show("ended")(inf.inflate(out))
    val corrupt = new Inflater(true)
    corrupt.setInput(Array[Byte](7, 1, 2))
    show("corrupt")(corrupt.inflate(out))
