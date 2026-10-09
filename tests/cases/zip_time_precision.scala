//> using platform jvm
// A time keeps its unit as the JDK's `FileTime` does: a zip entry's microseconds survive in memory,
// through `ZipOutputStream` (an NTFS field past 2038, the JDK's bytes), `ZipInputStream` and
// `ZipFile`, and are read intact from an archive the JDK wrote; two times of different units
// compare, equal and hash by the instant they stand for, a conversion saturates, and the text is
// the ISO instant with its fraction. The second review's `ZipPrecision` and `ZipCross`.
import java.io.{ByteArrayInputStream, ByteArrayOutputStream}
import java.nio.file.Files
import java.nio.file.attribute.FileTime
import java.util.TimeZone
import java.security.MessageDigest
import java.util.HexFormat
import java.util.concurrent.TimeUnit.*
import java.util.zip.*

object Main:
  // The JDK's archive of an entry `micro` written at 4354819200123456 µs (`ZipCross`'s write).
  val jdkArchive =
    "504b030414000808080000002100000000000000000000000000050024006d6963726f0a002000000000000100180080961668b868380200000000000000800000000000000080730400504b07088b9ed9d30300000001000000504b01021400140008080800000021008b9ed9d303000000010000000500240000000000000000000000000000006d6963726f0a002000000000000100180080961668b868380200000000000000800000000000000080504b05060000000001000100570000005a0000000000"

  def main(args: Array[String]): Unit =
    TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
    val t = 4354819200123456L
    val e = new ZipEntry("micro")
    e.setLastModifiedTime(FileTime.from(t, MICROSECONDS))
    println("memory=" + e.getLastModifiedTime.to(MICROSECONDS))
    val b = new ByteArrayOutputStream()
    val out = new ZipOutputStream(b)
    out.putNextEntry(e)
    out.write(65)
    out.close()
    println("bytes=" + HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b.toByteArray)))
    val in = new ZipInputStream(new ByteArrayInputStream(b.toByteArray))
    println("stream=" + in.getNextEntry().getLastModifiedTime.to(MICROSECONDS))
    in.close()
    val path = Files.createTempFile("teq-micro", ".zip")
    Files.write(path, b.toByteArray)
    val z = new ZipFile(path.toFile)
    println("file=" + z.getEntry("micro").getLastModifiedTime.to(MICROSECONDS))
    z.close()

    Files.write(path, HexFormat.of().parseHex(jdkArchive))
    val jz = new ZipFile(path.toFile)
    println("the JDK's archive, file=" + jz.getEntry("micro").getLastModifiedTime.to(MICROSECONDS))
    jz.close()
    val jin = new ZipInputStream(Files.newInputStream(path))
    println("the JDK's archive, stream=" + jin.getNextEntry().getLastModifiedTime.to(MICROSECONDS))
    jin.close()
    Files.delete(path)

    val micro = FileTime.from(t, MICROSECONDS)
    val millis = FileTime.fromMillis(t / 1000)
    println(s"$micro ${micro.toMillis} ${micro.to(NANOSECONDS)} ${micro.to(SECONDS)} ${micro.to(DAYS)}")
    println(s"${micro.compareTo(millis)} ${millis.compareTo(micro)} ${micro == millis}")
    val again = FileTime.from(t / 1000 * 1000, MICROSECONDS)
    println(s"${millis == again} ${millis.hashCode == again.hashCode} ${millis.compareTo(again)} ${millis.hashCode}")
    println(FileTime.from(-1, NANOSECONDS))
    println(FileTime.from(1, DAYS))
    println(FileTime.from(-62135596801L, SECONDS))
    println(FileTime.from(253402300800L, SECONDS))
    println(FileTime.from(1500, MILLISECONDS))
    println(FileTime.from(Long.MaxValue, DAYS).to(SECONDS))
    println(FileTime.from(Long.MaxValue, DAYS).compareTo(FileTime.from(Long.MaxValue, SECONDS)))
    println(FileTime.from(Long.MinValue, HOURS).compareTo(FileTime.from(Long.MinValue, MINUTES)))
    println(try FileTime.from(1, null) catch case e: NullPointerException => e.getMessage)
