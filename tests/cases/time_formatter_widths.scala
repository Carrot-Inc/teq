//> using platform jvm
// `DateTimeFormatter.ofPattern` as the JDK's builder reads a pattern: each letter's count a number
// of a width, a short, full or narrow name, a fraction or an offset of a form, the counts it refuses
// refused with its messages; quoted text; optional sections printed only where their fields are;
// reserved characters; `withZone` applied to a temporal that is an instant and to no other.
import java.time.*
import java.time.format.DateTimeFormatter

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName + ": " + e.getMessage
    println(s"$name = $r")

  def main(args: Array[String]): Unit =
    val zoned = ZonedDateTime.of(2024, 7, 3, 9, 4, 6, 120456789, ZoneOffset.of("+05:30:15"))
    val utc = ZonedDateTime.of(2024, 12, 29, 23, 0, 0, 0, ZoneOffset.UTC)
    val patterns = List("M MM MMM MMMM MMMMM", "L LL LLL LLLL LLLLL", "E EE EEE EEEE EEEEE", "d dd", "H HH h hh k kk a", "m mm s ss",
      "S SS SSS SSSSSS SSSSSSSSS", "n nnn nnnnnnnnnnnn", "y yy yyy yyyy yyyyy", "u uu uuuu",
      "X XX XXX XXXX XXXXX", "x xx xxx xxxx xxxxx", "Z ZZ ZZZ ZZZZ ZZZZZ",
      "MMMMMM", "EEEEEE", "aa", "ddd", "HHH", "XXXXXX", "xxxxxx", "ZZZZZZ", "SSSSSSSSSS",
      "'at' HH 'o''clock' ''", "yyyy[ 'and' HH:mm]", "HH[:mm[:ss]]", "yyyy]", "yyyy{", "MM#", "uuuu-MM-dd'T'HH:mm:ssXXX")
    for p <- patterns do
      show(p)(DateTimeFormatter.ofPattern(p).format(zoned))
    for p <- List("X XX XXX XXXX XXXXX", "x xx xxx xxxx xxxxx", "Z ZZZZ ZZZZZ") do show("utc " + p)(DateTimeFormatter.ofPattern(p).format(utc))
    val date = LocalDate.of(2024, 2, 29)
    show("a date")(DateTimeFormatter.ofPattern("EEEE d MMMM uuuu").format(date))
    show("a date's hour")(DateTimeFormatter.ofPattern("HH").format(date))
    show("a date, optional time")(DateTimeFormatter.ofPattern("uuuu-MM-dd[ HH:mm]").format(date))
    val local = LocalDateTime.of(2024, 1, 2, 3, 4, 5, 6000000)
    show("a local time")(DateTimeFormatter.ofPattern("uuuu-MM-dd HH:mm:ss.SSS").format(local))
    show("a local time's offset")(DateTimeFormatter.ofPattern("HH XXX").format(local))
    show("a local time with a zone")(DateTimeFormatter.ofPattern("HH:mm").withZone(ZoneOffset.UTC).format(local))
    val instant = zoned.toInstant
    show("an instant's fraction")(DateTimeFormatter.ofPattern("SSS").format(instant))
    show("an instant's hour")(DateTimeFormatter.ofPattern("HH").format(instant))
    show("an instant with a zone")(DateTimeFormatter.ofPattern("uuuu-MM-dd HH:mm:ss XXX").withZone(ZoneOffset.of("-03:00")).format(instant))
    show("a zoned time with a zone")(DateTimeFormatter.ofPattern("HH:mm:ss XXXXX").withZone(ZoneOffset.UTC).format(zoned))
    show("the same zone")(DateTimeFormatter.ofPattern("HH:mm XXX").withZone(ZoneOffset.of("+05:30:15")).format(zoned))
    show("a long year")(DateTimeFormatter.ofPattern("yyyy uuuu").format(ZonedDateTime.of(12345, 1, 1, 0, 0, 0, 0, ZoneOffset.UTC)))
    show("an unclosed quote")(DateTimeFormatter.ofPattern("'open"))
