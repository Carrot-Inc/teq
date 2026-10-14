//> using platform jvm
// An archive entry's DOS time is local time in the default `TimeZone` (`ZipUtils`, `ZoneId.systemDefault()`),
// written and read in the zone the program set, as the JDK's are: an entry written in one zone and read in another
// moves by the difference. `TimeZone.getDefault()` is the machine's zone until `setDefault`, `ZoneId.systemDefault()`
// follows it, and a fixed offset's `getOffset` is its own. JavaScript has no archives, so this runs in the
// interpreter and on the JVM alone.
import java.util.TimeZone
import java.util.zip.{ZipEntry, ZipInputStream, ZipOutputStream}

object Main:
  def write(time: Long): Array[Byte] =
    val out = new java.io.ByteArrayOutputStream()
    val zip = new ZipOutputStream(out)
    val entry = new ZipEntry("x")
    entry.setTime(time)
    zip.putNextEntry(entry)
    zip.closeEntry()
    zip.close()
    out.toByteArray()

  def read(bytes: Array[Byte]): Long =
    val in = new ZipInputStream(new java.io.ByteArrayInputStream(bytes))
    try in.getNextEntry().getTime() finally in.close()

  def main(args: Array[String]): Unit =
    val t = 1700000000000L
    TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
    val utc = write(t)
    for id <- List("UTC", "GMT+05:00", "GMT-03:30", "GMT") do
      TimeZone.setDefault(TimeZone.getTimeZone(id))
      println(id + " " + (read(utc) - t))
    TimeZone.setDefault(TimeZone.getTimeZone("GMT+05:00"))
    val east = write(t)
    println(read(east) - t)
    TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
    println(read(east) - t)
    println(List(TimeZone.getTimeZone("GMT+05:00").getID, TimeZone.getTimeZone("GMT+05:00").getOffset(t), TimeZone.getTimeZone("GMT-03:30").getOffset(t)))
    TimeZone.setDefault(TimeZone.getTimeZone("GMT-03:30"))
    println(java.time.ZoneId.systemDefault())
    TimeZone.setDefault(null)
    println(TimeZone.getDefault().getID == java.time.ZoneId.systemDefault().getId)
