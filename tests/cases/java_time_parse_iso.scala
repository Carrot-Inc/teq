//> using platform jvm
// java.time as a script's shared code reads it: `LocalTime`, `Month` and `DayOfWeek`, the ISO
// formatters, a pattern's parse into a date, a time and a date-time (names, two-digit years, AM and
// PM), `Instant.parse`, `truncatedTo`, `plus` and `minus` by a unit, `atOffset`, the units' `between`
// of dates, zoned date-times and instants, a duration's parts, and the JDK's messages for a text that
// does not parse and a unit an instant does not take.
import java.time.*
import java.time.format.{DateTimeFormatter, DateTimeParseException}
import java.time.temporal.{ChronoUnit, UnsupportedTemporalTypeException}

object Main:
  def attempt(what: String)(body: => Any): Unit =
    try println(what + ": " + body)
    catch
      case e: DateTimeParseException => println(what + ": DateTimeParseException " + e.getMessage + " @" + e.getErrorIndex)
      case e: DateTimeException => println(what + ": " + e.getClass.getSimpleName + " " + e.getMessage)

  def main(args: Array[String]): Unit =
    val t = LocalTime.of(10, 15)
    println(t)
    println(LocalTime.of(10, 15, 30, 500000000))
    println(DateTimeFormatter.ISO_LOCAL_TIME.format(t))
    println(DateTimeFormatter.ISO_LOCAL_TIME.format(LocalTime.of(10, 15, 30, 500000000)))
    println(DateTimeFormatter.ofPattern("hh:mm a").format(LocalTime.of(15, 45)))
    println(DateTimeFormatter.ofPattern("HH:mm:ss.SSS").format(LocalTime.of(9, 5, 7, 123456789)))
    println(s"${LocalTime.parse("10:15")} ${LocalTime.parse("10:15:30.123")} ${LocalTime.parse("03:45 PM", DateTimeFormatter.ofPattern("hh:mm a"))}")
    println(s"${t.getHour} ${t.getMinute} ${t.isBefore(LocalTime.NOON)} ${LocalTime.MIDNIGHT} ${t.toSecondOfDay}")
    attempt("bad time")(LocalTime.parse("25:00"))

    println(s"${Month.JANUARY.getValue} ${Month.of(12)} ${Month.valueOf("MAY")} ${Month.values().length}")
    val leap = LocalDate.of(2024, Month.FEBRUARY, 29)
    println(s"$leap ${leap.getMonth} ${leap.getDayOfWeek} ${leap.getDayOfWeek.getValue} ${DayOfWeek.of(7)}")
    println(LocalDate.of(2024, 1, 1).getDayOfWeek)

    println(LocalDate.parse("2024-03-05"))
    println(LocalDate.parse("March 05, 2024", DateTimeFormatter.ofPattern("MMMM dd, yyyy")))
    println(LocalDate.parse("03/05/2024", DateTimeFormatter.ofPattern("MM/dd/yyyy")))
    println(LocalDate.parse("Mar 5, 24", DateTimeFormatter.ofPattern("MMM d, yy")))
    println(LocalDate.parse("02/30/2023", DateTimeFormatter.ofPattern("MM/dd/yyyy")))
    attempt("strict")(LocalDate.parse("2023-02-30"))
    attempt("index")(LocalDate.parse("2024-3-05"))
    attempt("trailing")(LocalDate.parse("2024-03-05x"))
    attempt("name")(LocalDate.parse("Marc 05, 2024", DateTimeFormatter.ofPattern("MMMM dd, yyyy")))
    println(DateTimeFormatter.ISO_DATE.format(leap) + " " + DateTimeFormatter.ISO_LOCAL_DATE.format(leap))
    println(leap.atStartOfDay(ZoneOffset.UTC).toInstant)

    val dt = LocalDateTime.of(2024, 3, 5, 14, 7)
    println(DateTimeFormatter.ISO_LOCAL_DATE_TIME.format(dt))
    println(LocalDateTime.parse("03/05/2024 2:07 PM", DateTimeFormatter.ofPattern("MM/dd/yyyy h:mm a")))
    println(DateTimeFormatter.ofPattern("EEE MMMM dd, yyyy").format(dt))

    val i = Instant.parse("2024-03-05T10:15:30Z")
    println(s"$i ${Instant.parse("2024-03-05T10:15:30.250Z")} ${DateTimeFormatter.ISO_INSTANT.format(i)}")
    println(s"${i.truncatedTo(ChronoUnit.DAYS)} ${i.truncatedTo(ChronoUnit.HOURS)}")
    println(s"${i.minus(3, ChronoUnit.HOURS)} ${i.plus(2, ChronoUnit.DAYS)} ${i.minus(90, ChronoUnit.MINUTES)}")
    println(s"${i.atOffset(ZoneOffset.UTC).toLocalDate} ${i.atOffset(ZoneOffset.ofHours(-12)).toLocalDate}")
    attempt("months")(i.minus(1, ChronoUnit.MONTHS))
    attempt("truncate")(i.truncatedTo(ChronoUnit.WEEKS))
    attempt("bad instant")(Instant.parse("2024-03-05 10:15:30Z"))

    val utc = ZoneOffset.UTC
    println(ChronoUnit.DAYS.between(LocalDate.of(2024, 1, 1), LocalDate.of(2024, 12, 31)))
    println(ChronoUnit.MONTHS.between(i.atZone(utc), Instant.parse("2025-01-04T00:00:00Z").atZone(utc)))
    println(ChronoUnit.MONTHS.between(LocalDate.of(2024, 1, 31), LocalDate.of(2024, 2, 29)))
    println(ChronoUnit.HOURS.between(i, Instant.parse("2024-03-06T09:00:00Z")))
    val d = Duration.ofSeconds(3725)
    println(s"${d.toHours} ${d.toMinutesPart} ${d.toSecondsPart}")
