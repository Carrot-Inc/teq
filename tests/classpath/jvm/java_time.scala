// jars: scala-library
// java.time of the JDK on the JVM target, the program of tests/classpath/js/java_time.scala: the
// classes the application names, built, parsed, formatted, moved, compared and converted.
// The zone rules are the jar's default provider (UTC, GMT and fixed offsets); a region id
// needs the tzdb jar registered by the program.
import java.time.*
import java.time.format.{DateTimeFormatter, DateTimeParseException}
import java.time.temporal.{ChronoField, ChronoUnit, TemporalAdjusters}
import java.util.Locale

object Main:
  def main(args: Array[String]): Unit =
    val d = LocalDate.of(2024, 2, 29)
    println(d)
    println("" + d.getYear + " " + d.getMonthValue + " " + d.getDayOfMonth + " " + d.getMonth + " " + d.getDayOfWeek + " " + d.getDayOfYear)
    println("" + d.isLeapYear + " " + d.lengthOfMonth + " " + d.lengthOfYear)
    println("" + d.plusDays(1) + " " + d.minusDays(60) + " " + d.plusMonths(12) + " " + d.plusYears(1) + " " + d.plusWeeks(2))
    println("" + d.plus(3, ChronoUnit.MONTHS) + " " + d.minus(1, ChronoUnit.YEARS) + " " + d.plus(Period.ofDays(10)))
    println("" + d.withDayOfMonth(1) + " " + d.withMonth(12) + " " + d.withYear(2000) + " " + d.withDayOfYear(365))
    println("" + d.`with`(TemporalAdjusters.firstDayOfMonth) + " " + d.`with`(TemporalAdjusters.lastDayOfMonth) + " " + d.`with`(TemporalAdjusters.next(DayOfWeek.MONDAY)) + " " + d.`with`(TemporalAdjusters.firstDayOfNextMonth))
    println("" + d.toEpochDay + " " + LocalDate.ofEpochDay(19782) + " " + LocalDate.ofYearDay(2023, 100) + " " + LocalDate.ofEpochDay(0) + " " + LocalDate.MIN + " " + LocalDate.MAX)
    println("" + LocalDate.parse("2023-12-31") + " " + LocalDate.parse("31/12/2023", DateTimeFormatter.ofPattern("dd/MM/yyyy")))
    println("" + d.format(DateTimeFormatter.ISO_LOCAL_DATE) + " " + d.format(DateTimeFormatter.ISO_DATE) + " " + d.format(DateTimeFormatter.BASIC_ISO_DATE) + " " + d.format(DateTimeFormatter.ofPattern("MM/dd/yyyy")) + " " + d.format(DateTimeFormatter.ofPattern("M/d/yy")) + " " + d.format(DateTimeFormatter.ofPattern("yyyyMMdd")))
    println("" + d.format(DateTimeFormatter.ofPattern("MMMM dd, yyyy", Locale.ENGLISH)) + " | " + d.format(DateTimeFormatter.ofPattern("EEE MMM d, yyyy", Locale.ENGLISH)) + " | " + d.format(DateTimeFormatter.ofPattern("EEEE", Locale.ENGLISH)) + " | " + d.format(DateTimeFormatter.ofPattern("MMMM EEEE", Locale.ROOT)))
    val d2 = LocalDate.of(2025, 1, 15)
    println("" + d.isBefore(d2) + " " + d.isAfter(d2) + " " + d.isEqual(d2) + " " + d.compareTo(d2) + " " + (d == LocalDate.of(2024, 2, 29)) + " " + d.equals(d2) + " " + d.hashCode + " " + d2.hashCode)
    println("" + d.until(d2, ChronoUnit.DAYS) + " " + ChronoUnit.MONTHS.between(d, d2) + " " + d.until(d2) + " " + Period.between(d, d2) + " " + ChronoUnit.WEEKS.between(d, d2))
    println("" + List(d2, d, LocalDate.of(2024, 1, 1)).sortWith(_.isBefore(_)) + " " + List(d2, d).minBy(_.toEpochDay) + " " + List(d2, d).maxBy(_.toEpochDay))
    println("" + d.get(ChronoField.DAY_OF_WEEK) + " " + d.getLong(ChronoField.EPOCH_DAY) + " " + d.get(ChronoField.ALIGNED_WEEK_OF_YEAR) + " " + d.isSupported(ChronoField.HOUR_OF_DAY) + " " + d.range(ChronoField.DAY_OF_MONTH))

    val t = LocalTime.of(13, 45, 30, 123456789)
    println("" + t + " " + t.getHour + " " + t.getMinute + " " + t.getSecond + " " + t.getNano + " " + LocalTime.of(9, 5) + " " + LocalTime.MIDNIGHT + " " + LocalTime.NOON + " " + LocalTime.MIN + " " + LocalTime.MAX)
    println("" + t.plusHours(12) + " " + t.minusMinutes(50) + " " + t.plusSeconds(3600) + " " + t.plusNanos(1000) + " " + t.plus(90, ChronoUnit.MINUTES) + " " + t.minus(Duration.ofHours(14)))
    println("" + t.truncatedTo(ChronoUnit.SECONDS) + " " + t.truncatedTo(ChronoUnit.MINUTES) + " " + t.truncatedTo(ChronoUnit.HOURS) + " " + t.truncatedTo(ChronoUnit.MILLIS) + " " + t.withHour(0) + " " + t.withNano(0) + " " + t.withSecond(0))
    println("" + t.toSecondOfDay + " " + t.toNanoOfDay + " " + LocalTime.ofSecondOfDay(3661) + " " + LocalTime.ofNanoOfDay(1L) + " " + LocalTime.parse("08:30") + " " + LocalTime.parse("08:30:15.5") + " " + LocalTime.parse("23:59:59.999999999"))
    println("" + t.format(DateTimeFormatter.ISO_LOCAL_TIME) + " " + t.format(DateTimeFormatter.ofPattern("HH:mm")) + " " + t.format(DateTimeFormatter.ofPattern("HH:mm:ss.SSS")) + " " + t.format(DateTimeFormatter.ofPattern("hh:mm a", Locale.ENGLISH)) + " " + t.format(DateTimeFormatter.ofPattern("h:mm a", Locale.ENGLISH)) + " " + LocalTime.of(0, 7).format(DateTimeFormatter.ofPattern("hh:mm a", Locale.ENGLISH)))
    println("" + LocalTime.parse("10:15", DateTimeFormatter.ofPattern("HH:mm[:ss]")) + " " + LocalTime.parse("10:15:42", DateTimeFormatter.ofPattern("HH:mm[:ss]")) + " " + t.isBefore(LocalTime.NOON) + " " + t.isAfter(LocalTime.NOON) + " " + t.compareTo(LocalTime.NOON) + " " + t.hashCode + " " + ChronoUnit.MINUTES.between(LocalTime.NOON, t) + " " + Duration.between(LocalTime.NOON, t))

    val dt = LocalDateTime.of(2024, 2, 29, 13, 45, 30)
    println("" + dt + " " + LocalDateTime.of(d, t) + " " + d.atTime(t) + " " + d.atTime(8, 0) + " " + d.atStartOfDay + " " + t.atDate(d2) + " " + dt.toLocalDate + " " + dt.toLocalTime)
    println("" + dt.plusDays(1).plusHours(11) + " " + dt.minusMonths(2) + " " + dt.plus(Duration.ofMinutes(15)) + " " + dt.truncatedTo(ChronoUnit.DAYS) + " " + dt.truncatedTo(ChronoUnit.HOURS) + " " + dt.withHour(23).withMinute(1) + " " + dt.getDayOfWeek + " " + dt.getDayOfYear)
    println("" + LocalDateTime.parse("2024-02-29T13:45:30") + " " + LocalDateTime.parse("2024-02-29T13:45:30.250") + " " + LocalDateTime.parse("29/02/2024 13:45", DateTimeFormatter.ofPattern("dd/MM/yyyy HH:mm")) + " " + LocalDateTime.ofEpochSecond(1709214330L, 0, ZoneOffset.UTC))
    println("" + dt.format(DateTimeFormatter.ISO_LOCAL_DATE_TIME) + " " + dt.format(DateTimeFormatter.ISO_DATE_TIME) + " " + dt.format(DateTimeFormatter.ofPattern("MM/dd/yyyy h:mm a", Locale.ENGLISH)) + " " + dt.format(DateTimeFormatter.ofPattern("MMM. d, yyyy h:mm a", Locale.ENGLISH)) + " " + dt.format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss")))
    println("" + dt.isBefore(dt.plusNanos(1)) + " " + dt.compareTo(dt) + " " + (dt == LocalDateTime.of(2024, 2, 29, 13, 45, 30)) + " " + dt.hashCode + " " + ChronoUnit.HOURS.between(dt, dt.plusDays(2)) + " " + dt.until(dt.plusDays(2), ChronoUnit.MINUTES) + " " + Duration.between(dt, dt.plusDays(1).plusSeconds(1)))

    val i = Instant.ofEpochMilli(1709214330500L)
    println("" + i + " " + i.toEpochMilli + " " + i.getEpochSecond + " " + i.getNano + " " + Instant.ofEpochSecond(1709214330L) + " " + Instant.ofEpochSecond(1709214330L, 999999999L) + " " + Instant.EPOCH + " " + Instant.MIN + " " + Instant.MAX)
    println("" + Instant.parse("2024-02-29T13:45:30Z") + " " + Instant.parse("2024-02-29T13:45:30.123456Z") + " " + Instant.parse("2024-02-29T13:45:30.5Z").toEpochMilli + " " + i.plusSeconds(60) + " " + i.minusMillis(500) + " " + i.plus(1, ChronoUnit.DAYS) + " " + i.plus(Duration.ofHours(3)) + " " + i.minus(2, ChronoUnit.HOURS))
    println("" + i.truncatedTo(ChronoUnit.SECONDS) + " " + i.truncatedTo(ChronoUnit.MINUTES) + " " + i.truncatedTo(ChronoUnit.DAYS) + " " + i.isBefore(Instant.EPOCH) + " " + i.isAfter(Instant.EPOCH) + " " + i.compareTo(Instant.EPOCH) + " " + (i == Instant.ofEpochMilli(1709214330500L)) + " " + i.hashCode + " " + Instant.EPOCH.hashCode)
    println("" + ChronoUnit.SECONDS.between(Instant.EPOCH, i) + " " + Duration.between(Instant.EPOCH, i) + " " + i.until(i.plusMillis(1500), ChronoUnit.MILLIS) + " " + i.getLong(ChronoField.INSTANT_SECONDS) + " " + i.get(ChronoField.MILLI_OF_SECOND) + " " + DateTimeFormatter.ISO_INSTANT.format(i) + " " + List(i, Instant.EPOCH, i.minusSeconds(1)).sortWith(_.isBefore(_)))

    val utc = ZoneOffset.UTC
    val plus2 = ZoneOffset.ofHours(2)
    val minus530 = ZoneOffset.ofHoursMinutes(-5, -30)
    println("" + utc + " " + plus2 + " " + minus530 + " " + ZoneOffset.of("+01:00") + " " + ZoneOffset.ofTotalSeconds(3600) + " " + plus2.getTotalSeconds + " " + plus2.getId + " " + utc.getId + " " + ZoneOffset.MIN + " " + ZoneOffset.MAX + " " + plus2.hashCode + " " + plus2.compareTo(utc))
    println("" + ZoneId.of("UTC") + " " + ZoneId.of("Z") + " " + ZoneId.of("+02:00") + " " + ZoneId.of("GMT") + " " + ZoneId.of("UTC").getId + " " + ZoneId.of("UTC").getRules.getOffset(i) + " " + ZoneId.of("UTC").normalized + " " + ZoneId.of("+02:00").getRules.isFixedOffset)
    val z = dt.atZone(utc)
    val z2 = dt.atZone(plus2)
    println("" + z + " " + z2 + " " + z.toInstant + " " + z2.toInstant + " " + z.toEpochSecond + " " + z2.toEpochSecond + " " + z.getOffset + " " + z2.getZone + " " + z2.toLocalDateTime + " " + z2.toLocalDate)
    println("" + i.atZone(utc) + " " + i.atZone(plus2) + " " + i.atZone(minus530) + " " + i.atOffset(plus2) + " " + i.atZone(plus2).toLocalDateTime + " " + LocalDateTime.ofInstant(i, plus2) + " " + LocalDateTime.ofInstant(i, minus530).toLocalDate + " " + z2.withZoneSameInstant(utc) + " " + z2.withZoneSameLocal(minus530))
    println("" + ZonedDateTime.of(d, t, plus2) + " " + ZonedDateTime.of(2024, 2, 29, 1, 2, 3, 4, utc) + " " + ZonedDateTime.parse("2024-02-29T13:45:30+02:00") + " " + ZonedDateTime.parse("2024-02-29T13:45:30Z[UTC]") + " " + ZonedDateTime.ofInstant(i, utc) + " " + z2.plusDays(1) + " " + z2.minusHours(3) + " " + z2.truncatedTo(ChronoUnit.HOURS))
    println("" + z.format(DateTimeFormatter.ISO_ZONED_DATE_TIME) + " " + z2.format(DateTimeFormatter.ISO_OFFSET_DATE_TIME) + " " + z2.format(DateTimeFormatter.ISO_DATE_TIME) + " " + z2.format(DateTimeFormatter.ISO_INSTANT) + " " + z2.format(DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm xxx")) + " " + z2.format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ssXXX")) + " " + z.isBefore(z2) + " " + z.compareTo(z2) + " " + z.isEqual(z2.plusHours(2)))
    val o = dt.atOffset(minus530)
    println("" + o + " " + o.toInstant + " " + o.toZonedDateTime + " " + o.withOffsetSameInstant(utc) + " " + o.toEpochSecond + " " + o.getOffset + " " + OffsetDateTime.parse("2024-02-29T13:45:30-05:30") + " " + OffsetDateTime.of(dt, plus2) + " " + i.atOffset(utc).format(DateTimeFormatter.ISO_OFFSET_DATE_TIME) + " " + o.hashCode + " " + o.isAfter(z.toOffsetDateTime) + " " + o.compareTo(z.toOffsetDateTime))

    val dur = Duration.ofSeconds(3725, 500000000L)
    println("" + dur + " " + dur.getSeconds + " " + dur.getNano + " " + dur.toMillis + " " + dur.toMinutes + " " + dur.toHours + " " + dur.toDays + " " + dur.toNanos + " " + dur.toSeconds + " " + dur.toMinutesPart + " " + dur.toSecondsPart + " " + dur.toMillisPart)
    println("" + Duration.ofMillis(1500) + " " + Duration.ofMinutes(90) + " " + Duration.ofHours(25) + " " + Duration.ofDays(2) + " " + Duration.ofNanos(1) + " " + Duration.ZERO + " " + Duration.parse("PT1H30M") + " " + Duration.parse("P2DT3H4M5.6S") + " " + Duration.parse("-PT0.5S") + " " + Duration.of(3, ChronoUnit.HOURS))
    println("" + dur.plus(Duration.ofMillis(500)) + " " + dur.minusSeconds(3600) + " " + dur.multipliedBy(3) + " " + dur.dividedBy(2) + " " + dur.negated + " " + dur.abs + " " + dur.isNegative + " " + dur.isZero + " " + dur.compareTo(Duration.ofHours(1)) + " " + (dur == Duration.ofMillis(3725500)) + " " + dur.hashCode + " " + dur.plusDays(1) + " " + dur.toHoursPart)
    println("" + Duration.between(t, LocalTime.MAX) + " " + Duration.between(i, i.plusMillis(1)) + " " + i.plus(dur) + " " + t.plus(dur) + " " + dt.minus(dur) + " " + dur.addTo(dt) + " " + List(dur, Duration.ZERO, Duration.ofDays(-1)).sortWith(_.compareTo(_) < 0))

    val p = Period.of(1, 14, 40)
    println("" + p + " " + p.getYears + " " + p.getMonths + " " + p.getDays + " " + p.normalized + " " + p.toTotalMonths + " " + Period.ofDays(10) + " " + Period.ofWeeks(2) + " " + Period.ofMonths(3) + " " + Period.ofYears(1) + " " + Period.ZERO + " " + Period.parse("P1Y2M3D") + " " + Period.parse("-P1M") + " " + Period.parse("P2W"))
    println("" + p.plusDays(5) + " " + p.minusMonths(2) + " " + p.multipliedBy(2) + " " + p.negated + " " + p.isNegative + " " + p.isZero + " " + (p == Period.of(1, 14, 40)) + " " + p.hashCode + " " + d.plus(p) + " " + d.minus(p) + " " + p.addTo(dt) + " " + Period.between(d2, d) + " " + Period.between(d, d2).getMonths)

    val y = Year.of(2024)
    println("" + y + " " + y.getValue + " " + y.isLeap + " " + Year.isLeap(1900) + " " + Year.isLeap(2000) + " " + Year.isLeap(2023) + " " + y.length + " " + y.atDay(60) + " " + y.atMonth(2) + " " + y.plusYears(1) + " " + Year.parse("1999") + " " + y.compareTo(Year.of(2000)) + " " + (y == Year.of(2024)) + " " + y.hashCode + " " + y.isBefore(Year.of(2025)) + " " + Year.from(d) + " " + y.atMonthDay(MonthDay.of(3, 1)))
    val ym = YearMonth.of(2024, 2)
    println("" + ym + " " + ym.getYear + " " + ym.getMonthValue + " " + ym.getMonth + " " + ym.lengthOfMonth + " " + ym.lengthOfYear + " " + ym.isLeapYear + " " + ym.atDay(29) + " " + ym.atEndOfMonth + " " + ym.plusMonths(11) + " " + ym.minusYears(1) + " " + ym.isValidDay(30) + " " + YearMonth.parse("2023-12") + " " + YearMonth.from(d) + " " + ym.compareTo(YearMonth.of(2024, 3)) + " " + ym.hashCode + " " + ym.format(DateTimeFormatter.ofPattern("MMMM yyyy", Locale.ENGLISH)) + " " + ym.format(DateTimeFormatter.ofPattern("MM/yyyy")))
    println("" + Month.FEBRUARY.length(true) + " " + Month.of(12) + " " + Month.JANUARY.plus(13) + " " + Month.MARCH.minus(3) + " " + Month.valueOf("MAY").getValue + " " + Month.values.length + " " + Month.APRIL.firstDayOfYear(false) + " " + Month.DECEMBER.firstMonthOfQuarter + " " + Month.MARCH.maxLength + " " + Month.FEBRUARY.minLength + " " + Month.JUNE.compareTo(Month.MAY) + " " + Month.JULY.ordinal)
    println("" + DayOfWeek.MONDAY + " " + DayOfWeek.of(7) + " " + DayOfWeek.SUNDAY.plus(1) + " " + DayOfWeek.MONDAY.minus(1) + " " + DayOfWeek.FRIDAY.getValue + " " + DayOfWeek.valueOf("TUESDAY") + " " + DayOfWeek.values.toList + " " + DayOfWeek.WEDNESDAY.ordinal + " " + DayOfWeek.from(d) + " " + MonthDay.of(2, 29) + " " + MonthDay.of(2, 29).isValidYear(2023) + " " + MonthDay.parse("--12-25").atYear(2024))

    println("" + ChronoUnit.DAYS.getDuration + " " + ChronoUnit.MONTHS.isDateBased + " " + ChronoUnit.HOURS.isTimeBased + " " + ChronoUnit.HALF_DAYS + " " + ChronoUnit.MICROS.getDuration + " " + ChronoUnit.DECADES.getDuration + " " + ChronoUnit.FOREVER.isDurationEstimated + " " + ChronoUnit.WEEKS.toString + " " + ChronoField.YEAR + " " + ChronoField.NANO_OF_SECOND.range + " " + ChronoField.HOUR_OF_DAY.getBaseUnit + " " + ChronoField.MONTH_OF_YEAR.isDateBased + " " + ChronoField.INSTANT_SECONDS.checkValidValue(5L) + " " + ChronoField.DAY_OF_MONTH.toString)
    for text <- List("2024-02-30", "2024-13-01", "not a date", "2024-2-9") do
      try println(LocalDate.parse(text))
      catch case e: DateTimeParseException => println(e.getMessage + " | " + e.getParsedString + " | " + e.getErrorIndex)
    try println(Instant.parse("2024-02-29"))
    catch case e: DateTimeParseException => println(e.getMessage)
    try println(LocalDate.of(2023, 2, 29))
    catch case e: DateTimeException => println(e.getMessage)
    try println(ZoneOffset.ofHours(19))
    catch case e: DateTimeException => println(e.getMessage)
    try println(LocalDate.of(2024, 1, 1).plus(1, ChronoUnit.HOURS))
    catch case e: DateTimeException => println(e.getMessage)
    println("" + Instant.now().isAfter(Instant.EPOCH) + " " + (LocalDate.now().getYear >= 2024) + " " + (LocalDateTime.now(ZoneOffset.UTC).getYear >= 2024) + " " + (ZonedDateTime.now(ZoneOffset.UTC).getOffset == ZoneOffset.UTC) + " " + (Clock.systemUTC().millis() > 0L) + " " + (LocalTime.now().getHour < 24))
