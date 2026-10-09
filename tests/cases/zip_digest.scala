//> using platform jvm
// Archives and digests against the JDK: a jar's entries read through `ZipFile` and through
// `ZipInputStream`; a zip written by `ZipOutputStream` (stored and deflated entries, a fixed time,
// a directory, a comment), its bytes the JDK's (their SHA-256), read back; gzip written and read;
// `Deflater` and `Inflater`, raw and with zlib's header, at three levels; CRC-32; MD5, SHA-1 and
// SHA-256 of text, of nothing and of a file's bytes, updated in pieces and reset by `digest`;
// `HexFormat` both ways. The files are under target/ of the working directory, removed at the end.
import java.io.{ByteArrayInputStream, ByteArrayOutputStream}
import java.nio.file.{Files, Paths}
import java.security.MessageDigest
import java.util.HexFormat
import java.util.zip.*
import scala.jdk.CollectionConverters.*

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def hex(b: Array[Byte]): String = HexFormat.of().formatHex(b)
  def sha256(b: Array[Byte]): String = hex(MessageDigest.getInstance("SHA-256").digest(b))

  // A local time in the machine's zone: 2010-01-01 00:00, as a script that wants fixed DOS fields
  // gives it.
  def fixedTime: Long =
    val probe = new ZipEntry("probe")
    var t = 1262304000000L - 14L * 3600 * 1000
    probe.setTime(t)
    while probe.getTime != t || dosFields(t) != (2010, 1, 1, 0, 0) do
      t += 15L * 60 * 1000
      probe.setTime(t)
    t

  def dosFields(t: Long): (Int, Int, Int, Int, Int) =
    val bytes = new ByteArrayOutputStream()
    val z = new ZipOutputStream(bytes)
    val e = new ZipEntry("x")
    e.setTime(t)
    z.putNextEntry(e)
    z.closeEntry()
    z.close()
    val b = bytes.toByteArray
    val dos = (b(10) & 0xff) | ((b(11) & 0xff) << 8) | ((b(12) & 0xff) << 16) | ((b(13) & 0xff) << 24)
    (((dos >> 25) & 0x7f) + 1980, (dos >> 21) & 0x0f, (dos >> 16) & 0x1f, (dos >> 11) & 0x1f, (dos >> 5) & 0x3f)

  def main(args: Array[String]): Unit =
    val dir = Paths.get("target/zip_digest")
    if Files.exists(dir) then Files.walk(dir).iterator.asScala.toList.reverse.foreach(Files.delete)
    Files.createDirectories(dir)
    try run(dir)
    finally Files.walk(dir).iterator.asScala.toList.reverse.foreach(Files.delete)

  def run(dir: java.nio.file.Path): Unit =
    // A jar read two ways.
    val jar = new ZipFile("tests/lsp/libs/lib/liba-1.0.jar")
    val entries = jar.entries().asScala.toList
    show("jar entries")(entries.map(e => s"${e.getName}:${e.getMethod}:${e.getSize}:${e.getCompressedSize}:${e.isDirectory}").mkString(" "))
    show("jar size")(jar.size())
    val first = entries.find(!_.isDirectory).get
    show("an entry's bytes")(sha256(jar.getInputStream(first).readAllBytes()))
    show("an entry's crc")(java.lang.Long.toHexString(first.getCrc))
    show("absent entry")(jar.getEntry("no/such/entry"))
    show("stream of entries")(jar.stream().count())
    jar.close()
    show("closed")(jar.size())
    val zin = new ZipInputStream(Files.newInputStream(Paths.get("tests/lsp/libs/lib/liba-1.0.jar")))
    var e = zin.getNextEntry()
    val streamed = scala.collection.mutable.ListBuffer.empty[String]
    while e != null do
      streamed += s"${e.getName}:${sha256(zin.readAllBytes()).take(12)}"
      e = zin.getNextEntry()
    show("streamed")(streamed.mkString(" "))
    show("not a zip")(new ZipFile("tests/cases/zip_digest.scala"))
    show("missing zip")(new ZipFile("target/zip_digest/absent.zip"))
    // A zip written: its bytes, then read back.
    val time = fixedTime
    val out = new ByteArrayOutputStream()
    val zip = new ZipOutputStream(out)
    zip.setComment("written by a test")
    val text = ("a line of text that repeats, " * 200).getBytes("UTF-8")
    val deflated = new ZipEntry("text/deflated.txt")
    deflated.setTime(time)
    zip.putNextEntry(deflated)
    zip.write(text)
    zip.closeEntry()
    val stored = new ZipEntry("text/stored.txt")
    stored.setTime(time)
    stored.setMethod(ZipEntry.STORED)
    stored.setSize(text.length)
    val crc = new CRC32()
    crc.update(text)
    stored.setCrc(crc.getValue)
    zip.putNextEntry(stored)
    zip.write(text, 0, 100)
    zip.write(text, 100, text.length - 100)
    zip.closeEntry()
    val folder = new ZipEntry("text/empty/")
    folder.setTime(time)
    zip.putNextEntry(folder)
    zip.closeEntry()
    val named = new ZipEntry("ünïcode-é.txt")
    named.setTime(time)
    named.setComment("an entry's comment")
    zip.putNextEntry(named)
    zip.write("é".getBytes("UTF-8"))
    show("duplicate")(zip.putNextEntry(new ZipEntry("text/stored.txt")))
    zip.close()
    val bytes = out.toByteArray
    show("zip bytes")(s"${bytes.length} ${sha256(bytes)}")
    show("sizes after the write")(s"${deflated.getSize} ${deflated.getCompressedSize} ${java.lang.Long.toHexString(deflated.getCrc)}")
    Files.write(dir.resolve("written.zip"), bytes)
    val back = new ZipFile(dir.resolve("written.zip").toFile)
    show("read back")(back.entries().asScala.map(e => s"${e.getName}:${e.getMethod}:${e.getSize}:${e.getTime == time}").mkString(" "))
    show("read back text")(new String(back.getInputStream(back.getEntry("text/deflated.txt")).readAllBytes(), "UTF-8") == new String(text, "UTF-8"))
    back.close()
    val wrong = new ZipOutputStream(new ByteArrayOutputStream())
    val bad = new ZipEntry("bad")
    bad.setMethod(ZipEntry.STORED)
    show("stored without a size")(wrong.putNextEntry(bad))
    // gzip.
    val gz = new ByteArrayOutputStream()
    val gzo = new GZIPOutputStream(gz)
    gzo.write(text)
    gzo.close()
    show("gzip bytes")(s"${gz.size()} ${sha256(gz.toByteArray)}")
    show("gunzip")(new String(new GZIPInputStream(new ByteArrayInputStream(gz.toByteArray)).readAllBytes(), "UTF-8") == new String(text, "UTF-8"))
    show("not gzip")(new GZIPInputStream(new ByteArrayInputStream(text)).read())
    // Deflater and Inflater.
    for level <- List(1, 6, 9); nowrap <- List(true, false) do
      val d = new Deflater(level, nowrap)
      d.setInput(text)
      d.finish()
      val buf = new Array[Byte](65536)
      val packed = new ByteArrayOutputStream()
      while !d.finished() do packed.write(buf, 0, d.deflate(buf))
      val inf = new Inflater(nowrap)
      inf.setInput(packed.toByteArray)
      val unpacked = new Array[Byte](text.length)
      val n = inf.inflate(unpacked)
      show(s"deflate level $level nowrap $nowrap")(s"${packed.size()} ${sha256(packed.toByteArray).take(16)} $n ${java.util.Arrays.equals(unpacked, text)} ${inf.finished()}")
    // CRC-32 and the digests.
    val c = new CRC32()
    c.update("abc".getBytes("UTF-8"))
    show("crc32")(java.lang.Long.toHexString(c.getValue))
    c.update('d')
    show("crc32 more")(java.lang.Long.toHexString(c.getValue))
    for alg <- List("MD5", "SHA-1", "SHA-256") do
      val md = MessageDigest.getInstance(alg)
      show(alg)(hex(md.digest("abc".getBytes("UTF-8"))))
      show(alg + " of nothing")(hex(md.digest()))
      md.update("a".getBytes("UTF-8"))
      md.update("bc".getBytes("UTF-8"), 0, 2)
      show(alg + " in pieces")(hex(md.digest()))
      show(alg + " of a file")(hex(md.digest(Files.readAllBytes(Paths.get("tests/lsp/libs/lib/liba-1.0.jar")))))
      show(alg + " length")(s"${md.getDigestLength} ${md.getAlgorithm}")
    show("unknown digest")(MessageDigest.getInstance("SHA-999"))
    show("hex upper")(HexFormat.of().withUpperCase().formatHex(Array[Byte](0, 15, -1, 127)))
    show("hex delimited")(HexFormat.ofDelimiter(":").formatHex(Array[Byte](1, 2, -3)))
    show("hex parsed")(HexFormat.of().parseHex("00ff7f80").mkString(","))
    show("hex bad")(HexFormat.of().parseHex("0g"))
