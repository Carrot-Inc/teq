//> using platform jvm
// An entry's times as the JDK keeps and writes them: `setTime` keeps the milliseconds a DOS time
// drops; an instant before 1980 or past 2099 in the machine's zone goes in an extended time stamp
// field, one past 2038 in an NTFS field, and both readers take the time back from it; the access
// and creation times written too; an extra field given by `setExtra` read for its times and
// written less the fields the stream writes itself. What is shown does not depend on the zone:
// lengths, the fields' ids and sizes, the times read back.
import java.io.{ByteArrayInputStream, ByteArrayOutputStream}
import java.nio.file.Files
import java.nio.file.attribute.FileTime
import java.util.zip.*

object Main:
  // The ids and sizes of an extra field's parts.
  def fields(extra: Array[Byte]): String =
    if extra == null then "none"
    else
      val parts = scala.collection.mutable.ListBuffer.empty[String]
      var off = 0
      while off + 4 <= extra.length do
        val id = (extra(off) & 0xff) | ((extra(off + 1) & 0xff) << 8)
        val sz = (extra(off + 2) & 0xff) | ((extra(off + 3) & 0xff) << 8)
        parts += f"0x$id%04x:$sz"
        off += 4 + sz
      parts.mkString(",")

  // The archive's bytes: its local header's extra field and its central directory's.
  def extras(zip: Array[Byte]): String =
    def u16(at: Int) = (zip(at) & 0xff) | ((zip(at + 1) & 0xff) << 8)
    val nameLen = u16(26)
    val locExtra = java.util.Arrays.copyOfRange(zip, 30 + nameLen, 30 + nameLen + u16(28))
    var cen = 0
    while !(zip(cen) == 0x50 && zip(cen + 1) == 0x4b && zip(cen + 2) == 1 && zip(cen + 3) == 2) do cen += 1
    val cenExtra = java.util.Arrays.copyOfRange(zip, cen + 46 + u16(cen + 28), cen + 46 + u16(cen + 28) + u16(cen + 30))
    s"loc ${fields(locExtra)} cen ${fields(cenExtra)}"

  def write(e: ZipEntry): Array[Byte] =
    val buf = new ByteArrayOutputStream()
    val out = new ZipOutputStream(buf)
    out.putNextEntry(e)
    out.write("data".getBytes)
    out.close()
    buf.toByteArray

  def main(args: Array[String]): Unit =
    val dir = Files.createTempDirectory("zip-times")
    for time <- List(0L, -1000L, 1262304001123L, 4036608000000L, 4354819200000L, 2000000000999L) do
      val e = new ZipEntry("t.txt")
      e.setTime(time)
      val zipped = write(e)
      val file = dir.resolve("t.zip")
      Files.write(file, zipped)
      val zf = new ZipFile(file.toFile)
      val fromFile = zf.getEntry("t.txt").getTime
      zf.close()
      val zin = new ZipInputStream(new ByteArrayInputStream(zipped))
      val fromStream = zin.getNextEntry().getTime
      zin.close()
      println(s"$time: kept ${e.getTime == time} length ${zipped.length} ${extras(zipped)} file ${fromFile == time || Math.abs(fromFile - time) < 2000} stream ${fromStream == time || Math.abs(fromStream - time) < 2000}")
    val e = new ZipEntry("all.txt")
    e.setLastModifiedTime(FileTime.fromMillis(1600000000000L))
    e.setLastAccessTime(FileTime.fromMillis(1600000100000L))
    e.setCreationTime(FileTime.fromMillis(1500000000000L))
    val all = write(e)
    val back = new ZipInputStream(new ByteArrayInputStream(all)).getNextEntry()
    println(s"three times: length ${all.length} ${extras(all)} read ${back.getLastModifiedTime.toMillis} ${back.getLastAccessTime.toMillis} ${back.getCreationTime.toMillis}")
    val far = new ZipEntry("far.txt")
    far.setLastModifiedTime(FileTime.fromMillis(4354819200000L))
    far.setLastAccessTime(FileTime.fromMillis(1600000100000L))
    val farZip = write(far)
    val farBack = new ZipInputStream(new ByteArrayInputStream(farZip)).getNextEntry()
    println(s"ntfs times: length ${farZip.length} ${extras(farZip)} read ${farBack.getLastModifiedTime.toMillis} ${farBack.getLastAccessTime.toMillis} ${farBack.getCreationTime}")
    val supplied = new ZipEntry("given.txt")
    supplied.setTime(1262304000000L)
    supplied.setExtra(Array[Byte](0x55, 0x54, 5, 0, 1, 0, 0x5e, 0x6f, 0x5f, 0x34, 0x12, 2, 0, 9, 9))
    println(s"given extra: time ${supplied.getTime} ${extras(write(supplied))}")
    Files.delete(dir.resolve("t.zip"))
    Files.delete(dir)
