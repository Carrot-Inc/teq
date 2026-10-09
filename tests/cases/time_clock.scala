//> using platform jvm
// The part of `java.time` a script reads the clock with, against the JDK: instants and durations
// printed and compared; dates and times of fixed offsets and of UTC printed and formatted with the
// patterns the scripts print (`HH:mm:ss`, `yyyyMMdd'T'HHmmss'Z'`, names, offsets, fractions); the
// machine's zone, whose rules are the system's, checked against `date`'s offset rather than
// printed, so that the expectation holds in any zone.
import java.time.*
import java.time.format.DateTimeFormatter

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def date(format: String): String =
    val p = new ProcessBuilder("date", format).start()
    val out = new String(p.getInputStream.readAllBytes(), "UTF-8").trim
    p.waitFor()
    out

  def main(args: Array[String]): Unit =
    val epoch = Instant.ofEpochSecond(1262304000L)
    show("instant")(epoch)
    show("instant millis")(Instant.ofEpochMilli(1262304000123L))
    show("instant micros")(Instant.ofEpochSecond(1262304000L, 123456000L))
    show("instant nanos")(Instant.ofEpochSecond(1262304000L, 123456789L))
    show("instant negative")(Instant.ofEpochMilli(-1L))
    show("epoch")(Instant.EPOCH)
    show("toEpochMilli")(Instant.ofEpochSecond(5L, 7000000L).toEpochMilli)
    show("compare")(epoch.isBefore(epoch.plusSeconds(1)) && epoch.plusMillis(5).isAfter(epoch))
    show("now")(Instant.now().getEpochSecond > 1262304000L)
    show("duration")(Duration.between(epoch, epoch.plusMillis(90061500L)))
    show("durations")(List(Duration.ZERO, Duration.ofMillis(1500), Duration.ofSeconds(-90), Duration.ofMillis(-1500), Duration.ofHours(25), Duration.ofNanos(1)).mkString(" "))
    show("duration millis")(Duration.ofSeconds(3, 500000000L).toMillis)
    val utc = ZonedDateTime.ofInstant(Instant.ofEpochSecond(1791529263L, 5000000L), ZoneOffset.UTC)
    show("utc")(utc)
    show("utc fields")(List(utc.getYear, utc.getMonthValue, utc.getDayOfMonth, utc.getHour, utc.getMinute, utc.getSecond, utc.getNano).mkString(","))
    show("utc stamp")(utc.format(DateTimeFormatter.ofPattern("yyyyMMdd'T'HHmmss'Z'")))
    show("utc clock")(utc.format(DateTimeFormatter.ofPattern("HH:mm:ss")))
    show("utc minute stamp")(utc.format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm'Z'")))
    show("utc dashed")(utc.format(DateTimeFormatter.ofPattern("yyyyMMdd-HHmmss")))
    show("names")(utc.format(DateTimeFormatter.ofPattern("EEE EEEE MMM MMMM d h a")))
    show("fraction")(utc.format(DateTimeFormatter.ofPattern("ss.SSS ss.SSSSSS")))
    show("quote")(utc.format(DateTimeFormatter.ofPattern("'o''clock' HH 'h'")))
    val east = ZonedDateTime.ofInstant(Instant.ofEpochSecond(1791529263L), ZoneOffset.ofHoursMinutes(5, 30))
    show("offset")(east)
    show("offset patterns")(east.format(DateTimeFormatter.ofPattern("X XX XXX x Z ZZZZZ")))
    show("utc patterns")(utc.format(DateTimeFormatter.ofPattern("X XXX x Z")))
    show("offset of")(List(ZoneOffset.of("+02:00"), ZoneOffset.of("-0730"), ZoneOffset.of("Z"), ZoneOffset.ofHours(-3)).mkString(" "))
    show("bad offset")(ZoneOffset.of("02:00"))
    show("same instant")(east.withZoneSameInstant(ZoneOffset.UTC).toInstant == east.toInstant)
    show("zone utc")(ZoneId.of("UTC"))
    show("zoned utc")(ZonedDateTime.ofInstant(Instant.ofEpochSecond(0L), ZoneId.of("UTC")))
    val local = LocalDateTime.of(2010, 1, 1, 0, 0)
    show("local")(local)
    show("local seconds")(LocalDateTime.of(2010, 1, 1, 0, 0, 5, 1000))
    show("local date")(LocalDate.of(2024, 2, 29))
    show("bad date")(LocalDate.of(2023, 2, 29))
    show("bad month")(LocalDate.of(2023, 13, 1))
    show("at offset")(local.atZone(ZoneOffset.ofHours(2)))
    show("epoch day")(LocalDate.of(2000, 3, 1).toEpochDay)
    show("plus days")(LocalDate.of(2024, 2, 28).plusDays(2))
    show("iso date")(LocalDate.of(2026, 10, 9).format(DateTimeFormatter.ISO_LOCAL_DATE))
    show("unknown letter")(DateTimeFormatter.ofPattern("yyyy-bb"))
    show("instant without a zone")(DateTimeFormatter.ofPattern("HH").format(epoch))
    show("instant with a zone")(DateTimeFormatter.ofPattern("HH:mm").withZone(ZoneOffset.UTC).format(epoch))
    show("date without a time")(DateTimeFormatter.ofPattern("HH").format(LocalDate.of(2020, 1, 1)))
    // The machine's zone: its offset now as `date` prints it, and the wall clock's minute.
    val before = date("+%z")
    val now = ZonedDateTime.now()
    show("machine offset")(Set(before, date("+%z")).contains(now.format(DateTimeFormatter.ofPattern("xx"))))
    show("machine zone")(now.getZone == ZoneId.systemDefault() && ZoneId.systemDefault().getId.nonEmpty)
    val clock = date("+%H:%M")
    show("machine clock")(Set(clock, now.format(DateTimeFormatter.ofPattern("HH:mm")), ZonedDateTime.now().format(DateTimeFormatter.ofPattern("HH:mm"))).size <= 2)
    show("local now")(LocalDateTime.now().getYear >= 2025)
