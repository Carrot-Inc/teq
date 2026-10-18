// The part of `java.time` a program under `teq interp` reads the clock with
// (src/interp/archive.rs): instants and durations, the machine's zone and fixed offsets, local and
// zoned dates and times, and formatters of patterns of numbers, names and offsets. A zone is the
// machine's, whose rules the system's time functions answer, or a fixed offset; another region's
// rules are not here. Left out of a build whose class path holds the package (scala-java-time,
// which a Scala.js program that runs with `java.time` links); JavaScript has no clock here.
package java.time:

  @js("$fail(\"UnsupportedOperationException\", \"the clock is not available here\")")
  def timeNow(): Array[Long]
  @js("$fail(\"UnsupportedOperationException\", \"the clock is not available here\")")
  def timeOffset(epochSecond: Long): Int
  @js("$fail(\"UnsupportedOperationException\", \"the clock is not available here\")")
  def timeLocalInstant(year: Int, month: Int, day: Int, hour: Int, minute: Int, second: Int): Long
  @js("$fail(\"UnsupportedOperationException\", \"the clock is not available here\")")
  def timeZoneName(): String

  // Days from 1970-01-01 and the civil date (Howard Hinnant's algorithms).
  private[time] def daysOf(y: Int, m: Int, d: Int): Long =
    val yy = (if m <= 2 then y - 1 else y).toLong
    val era = Math.floorDiv(yy, 400L)
    val yoe = yy - era * 400
    val mp = (m + 9) % 12
    val doy = (153 * mp + 2) / 5 + d - 1
    val doe = yoe * 365 + yoe / 4 - yoe / 100 + doy
    era * 146097 + doe - 719468
  private[time] def dateOf(days: Long): (Int, Int, Int) =
    val z = days + 719468
    val era = Math.floorDiv(z, 146097L)
    val doe = z - era * 146097
    val yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365
    val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
    val mp = (5 * doy + 2) / 153
    val d = (doy - (153 * mp + 2) / 5 + 1).toInt
    val m = (if mp < 10 then mp + 3 else mp - 9).toInt
    ((yoe + era * 400 + (if m <= 2 then 1 else 0)).toInt, m, d)
  private[time] def pad(n: Long, width: Int): String =
    val s = Math.abs(n).toString
    (if n < 0 then "-" else "") + ("0" * (width - s.length).max(0)) + s
  // `.SSS`, `.SSSSSS` or nine digits, as the JDK's ISO formats print a fraction.
  private[time] def fraction(nano: Int): String =
    if nano == 0 then ""
    else if nano % 1000000 == 0 then "." + pad(nano / 1000000, 3)
    else if nano % 1000 == 0 then "." + pad(nano / 1000, 6)
    else "." + pad(nano, 9)
  private[time] def checkDate(y: Int, m: Int, d: Int): Unit =
    if m < 1 || m > 12 then throw new DateTimeException("Invalid value for MonthOfYear (valid values 1 - 12): " + m)
    val leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
    val days = Array(31, if leap then 29 else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)(m - 1)
    if d < 1 || d > 31 then throw new DateTimeException("Invalid value for DayOfMonth (valid values 1 - 28/31): " + d)
    if d > days then
      throw new DateTimeException(if m == 2 && d == 29 then "Invalid date 'February 29' as '" + y + "' is not a leap year" else "Invalid date '" + MonthNames.names(m - 1).toUpperCase + " " + d + "'")
  private[time] def checkTime(h: Int, mi: Int, s: Int, n: Int): Unit =
    if h < 0 || h > 23 then throw new DateTimeException("Invalid value for HourOfDay (valid values 0 - 23): " + h)
    if mi < 0 || mi > 59 then throw new DateTimeException("Invalid value for MinuteOfHour (valid values 0 - 59): " + mi)
    if s < 0 || s > 59 then throw new DateTimeException("Invalid value for SecondOfMinute (valid values 0 - 59): " + s)
    if n < 0 || n > 999999999 then throw new DateTimeException("Invalid value for NanoOfSecond (valid values 0 - 999999999): " + n)

  private[time] object MonthNames:
    val names = Array("January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December")
    def lengthOf(month: Int, leap: Boolean): Int = Array(31, if leap then 29 else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)(month - 1)

  // What a formatter reads of a date, a time or an instant.
  trait TemporalAccessor:
    private[time] def fieldsOrNull: Fields

  // A temporal's fields as a formatter reads them: the date's, the time's, the nano of second (an
  // `Instant` has it without a time), the offset; each part there or not.
  private[time] final class Fields(val date: Boolean, val year: Int, val month: Int, val day: Int, val time: Boolean, val hour: Int, val minute: Int, val second: Int, val fraction: Boolean, val nano: Int, val offset: ZoneOffset, val zone: ZoneId):
    def dayOfWeek: Int = Math.floorMod(daysOf(year, month, day) + 3, 7L).toInt + 1

  final class Instant private[time] (seconds: Long, nanos: Int) extends Comparable[Instant], java.time.temporal.Temporal:
    def getEpochSecond: Long = seconds
    def getNano: Int = nanos
    def toEpochMilli: Long = Math.addExact(Math.multiplyExact(seconds, 1000L), (nanos / 1000000).toLong)
    def plusSeconds(s: Long): Instant = Instant.ofEpochSecond(seconds + s, nanos.toLong)
    def plusMillis(ms: Long): Instant = Instant.ofEpochSecond(seconds, nanos + ms * 1000000L)
    def plusNanos(ns: Long): Instant = Instant.ofEpochSecond(seconds, nanos + ns)
    def minusSeconds(s: Long): Instant = plusSeconds(-s)
    def minusMillis(ms: Long): Instant = plusMillis(-ms)
    def plus(d: Duration): Instant = Instant.ofEpochSecond(seconds + d.getSeconds, nanos.toLong + d.getNano)
    def minus(d: Duration): Instant = Instant.ofEpochSecond(seconds - d.getSeconds, nanos.toLong - d.getNano)
    def isBefore(other: Instant): Boolean = compareTo(other) < 0
    def isAfter(other: Instant): Boolean = compareTo(other) > 0
    def atZone(zone: ZoneId): ZonedDateTime = ZonedDateTime.ofInstant(this, zone)
    def atOffset(offset: ZoneOffset): OffsetDateTime = OffsetDateTime.ofInstant(this, offset)
    // The JDK's `plus(amount, unit)`: the units of a day and shorter; a longer one refused.
    def plus(amount: Long, unit: java.time.temporal.TemporalUnit): Instant =
      import java.time.temporal.ChronoUnit as U
      if unit == U.NANOS then plusNanos(amount)
      else if unit == U.MICROS then plusSeconds(amount / 1000000L).plusNanos((amount % 1000000L) * 1000L)
      else if unit == U.MILLIS then plusMillis(amount)
      else if unit == U.SECONDS then plusSeconds(amount)
      else if unit == U.MINUTES then plusSeconds(Math.multiplyExact(amount, 60L))
      else if unit == U.HOURS then plusSeconds(Math.multiplyExact(amount, 3600L))
      else if unit == U.HALF_DAYS then plusSeconds(Math.multiplyExact(amount, 43200L))
      else if unit == U.DAYS then plusSeconds(Math.multiplyExact(amount, 86400L))
      else throw new java.time.temporal.UnsupportedTemporalTypeException("Unsupported unit: " + unit)
    def minus(amount: Long, unit: java.time.temporal.TemporalUnit): Instant =
      if amount == Long.MinValue then plus(Long.MaxValue, unit).plus(1L, unit) else plus(-amount, unit)
    // The JDK's `truncatedTo`: down to a unit that divides a day.
    def truncatedTo(unit: java.time.temporal.TemporalUnit): Instant =
      if unit == java.time.temporal.ChronoUnit.NANOS then this
      else
        val d = unit.getDuration()
        if d.getSeconds > 86400L then throw new java.time.temporal.UnsupportedTemporalTypeException("Unit is too large to be used for truncation")
        val dur = d.toNanos
        if 86400000000000L % dur != 0L then throw new java.time.temporal.UnsupportedTemporalTypeException("Unit must divide into a standard day without remainder")
        val nod = Math.floorMod(seconds, 86400L) * 1000000000L + nanos
        plusNanos(Math.floorDiv(nod, dur) * dur - nod)
    // The JDK's `until`: whole units of a day and shorter from this instant to the other.
    def until(endExclusive: java.time.temporal.Temporal, unit: java.time.temporal.TemporalUnit): Long =
      import java.time.temporal.ChronoUnit as U
      val end = Instant.from(endExclusive)
      def secondsUntil: Long =
        var secs = Math.subtractExact(end.getEpochSecond, seconds)
        val ns = end.getNano - nanos
        if secs > 0 && ns < 0 then secs -= 1 else if secs < 0 && ns > 0 then secs += 1
        secs
      def nanosUntil: Long = Math.addExact(Math.multiplyExact(Math.subtractExact(end.getEpochSecond, seconds), 1000000000L), (end.getNano - nanos).toLong)
      if unit == U.NANOS then nanosUntil
      else if unit == U.MICROS then nanosUntil / 1000L
      else if unit == U.MILLIS then Math.subtractExact(end.toEpochMilli, toEpochMilli)
      else if unit == U.SECONDS then secondsUntil
      else if unit == U.MINUTES then secondsUntil / 60L
      else if unit == U.HOURS then secondsUntil / 3600L
      else if unit == U.HALF_DAYS then secondsUntil / 43200L
      else if unit == U.DAYS then secondsUntil / 86400L
      else throw new java.time.temporal.UnsupportedTemporalTypeException("Unsupported unit: " + unit)
    def compareTo(other: Instant): Int =
      val c = java.lang.Long.compare(seconds, other.getEpochSecond)
      if c != 0 then c else nanos - other.getNano
    private[time] def fieldsOrNull: Fields = new Fields(false, 0, 0, 0, false, 0, 0, 0, true, nanos, null, null)
    override def equals(other: Any): Boolean = other match
      case i: Instant => compareTo(i) == 0
      case _ => false
    override def hashCode: Int = java.lang.Long.hashCode(seconds) + 51 * nanos
    override def toString: String =
      val (y, m, d) = dateOf(Math.floorDiv(seconds, 86400L))
      val rest = Math.floorMod(seconds, 86400L).toInt
      pad(y, 4) + "-" + pad(m, 2) + "-" + pad(d, 2) + "T" + pad(rest / 3600, 2) + ":" + pad(rest / 60 % 60, 2) + ":" + pad(rest % 60, 2) + fraction(nanos) + "Z"

  object Instant:
    val EPOCH: Instant = new Instant(0L, 0)
    def now(): Instant =
      val t = timeNow()
      new Instant(t(0), t(1).toInt)
    def ofEpochSecond(epochSecond: Long): Instant = new Instant(epochSecond, 0)
    def ofEpochSecond(epochSecond: Long, nanoAdjustment: Long): Instant =
      new Instant(Math.addExact(epochSecond, Math.floorDiv(nanoAdjustment, 1000000000L)), Math.floorMod(nanoAdjustment, 1000000000L).toInt)
    def ofEpochMilli(epochMilli: Long): Instant = new Instant(Math.floorDiv(epochMilli, 1000L), (Math.floorMod(epochMilli, 1000L) * 1000000L).toInt)
    // `DateTimeFormatter.ISO_INSTANT`'s text: a UTC date and time, or one with an offset.
    def parse(text: CharSequence): Instant = java.time.format.DateTimeFormatter.ISO_INSTANT.parseInstant(text)
    def from(temporal: TemporalAccessor): Instant = temporal match
      case i: Instant => i
      case z: ZonedDateTime => z.toInstant
      case o: OffsetDateTime => o.toInstant
      case other => throw new DateTimeException("Unable to obtain Instant from TemporalAccessor: " + other + " of type " + other.getClass.getName)

  final class Duration private (seconds: Long, nanos: Int) extends Comparable[Duration]:
    def getSeconds: Long = seconds
    def toSeconds: Long = seconds
    def getNano: Int = nanos
    def toMillis: Long = Math.addExact(Math.multiplyExact(seconds, 1000L), (nanos / 1000000).toLong)
    def toNanos: Long = Math.addExact(Math.multiplyExact(seconds, 1000000000L), nanos.toLong)
    def toMinutes: Long = seconds / 60
    def toHours: Long = seconds / 3600
    def isNegative: Boolean = seconds < 0
    def isZero: Boolean = seconds == 0 && nanos == 0
    def toDays: Long = seconds / 86400
    def toDaysPart: Long = seconds / 86400
    def toHoursPart: Int = (toHours % 24).toInt
    def toMinutesPart: Int = (toMinutes % 60).toInt
    def toSecondsPart: Int = (seconds % 60).toInt
    def toMillisPart: Int = nanos / 1000000
    def toNanosPart: Int = nanos
    def plus(d: Duration): Duration = Duration.ofSeconds(Math.addExact(seconds, d.getSeconds), nanos.toLong + d.getNano)
    def minus(d: Duration): Duration = Duration.ofSeconds(Math.subtractExact(seconds, d.getSeconds), nanos.toLong - d.getNano)
    def plusSeconds(s: Long): Duration = if s == 0L then this else Duration.ofSeconds(Math.addExact(seconds, s), nanos.toLong)
    def plusMillis(ms: Long): Duration = if ms == 0L then this else plusSeconds(ms / 1000L).plusNanos((ms % 1000L) * 1000000L)
    def plusNanos(n: Long): Duration = if n == 0L then this else Duration.ofSeconds(Math.addExact(seconds, n / 1000000000L), nanos.toLong + n % 1000000000L)
    def minusNanos(n: Long): Duration = if n == Long.MinValue then plusNanos(Long.MaxValue).plusNanos(1L) else plusNanos(-n)
    // The JDK's `plus(amount, unit)`: days as 86400 seconds, a unit of an estimated length refused.
    def plus(amount: Long, unit: java.time.temporal.TemporalUnit): Duration =
      if unit == java.time.temporal.ChronoUnit.DAYS then plusSeconds(Math.multiplyExact(amount, 86400L))
      else if unit.isDurationEstimated() then throw new java.time.temporal.UnsupportedTemporalTypeException("Unit must not have an estimated duration")
      else if amount == 0L then this
      else if unit == java.time.temporal.ChronoUnit.NANOS then plusNanos(amount)
      else if unit == java.time.temporal.ChronoUnit.MICROS then plusSeconds((amount / 1000000000L) * 1000L).plusNanos((amount % 1000000000L) * 1000L)
      else if unit == java.time.temporal.ChronoUnit.MILLIS then plusMillis(amount)
      else if unit == java.time.temporal.ChronoUnit.SECONDS then plusSeconds(amount)
      else plusSeconds(Math.multiplyExact(unit.getDuration().getSeconds, amount))
    def negated(): Duration = multipliedBy(-1L)
    def abs(): Duration = if isNegative then negated() else this
    def multipliedBy(scalar: Long): Duration =
      if scalar == 0L then Duration.ZERO
      else if scalar == 1L then this
      else
        val total = BigInt(seconds) * 1000000000L + nanos
        Duration.ofNanosExact(total * scalar)
    def dividedBy(divisor: Long): Duration =
      if divisor == 0L then throw new ArithmeticException("Cannot divide by zero")
      else if divisor == 1L then this
      else
        val total = BigInt(seconds) * 1000000000L + nanos
        Duration.ofNanosExact(total / divisor)
    def compareTo(other: Duration): Int =
      val c = java.lang.Long.compare(seconds, other.getSeconds)
      if c != 0 then c else nanos - other.getNano
    override def equals(other: Any): Boolean = other match
      case d: Duration => compareTo(d) == 0
      case _ => false
    override def hashCode: Int = java.lang.Long.hashCode(seconds) + 51 * nanos
    override def toString: String =
      if seconds == 0 && nanos == 0 then "PT0S"
      else
        // As the JDK's: hours, minutes, then the seconds with their fraction, its trailing zeros dropped.
        val effective = if seconds < 0 && nanos > 0 then seconds + 1 else seconds
        val hours = effective / 3600
        val minutes = (effective % 3600) / 60
        val secs = effective % 60
        val sb = new java.lang.StringBuilder("PT")
        if hours != 0 then sb.append(hours).append('H')
        if minutes != 0 then sb.append(minutes).append('M')
        if secs == 0 && nanos == 0 && sb.length > 2 then sb.toString
        else
          if seconds < 0 && nanos > 0 && secs == 0 then sb.append("-0") else sb.append(secs)
          if nanos > 0 then
            val n = if seconds < 0 then 2 * 1000000000L - nanos else nanos + 1000000000L
            var frac = n.toString
            while frac.endsWith("0") do frac = frac.substring(0, frac.length - 1)
            sb.append('.').append(frac.substring(1))
          sb.append('S').toString

  object Duration:
    val ZERO: Duration = new Duration(0L, 0)
    def ofSeconds(seconds: Long): Duration = new Duration(seconds, 0)
    def ofSeconds(seconds: Long, nanoAdjustment: Long): Duration =
      new Duration(Math.addExact(seconds, Math.floorDiv(nanoAdjustment, 1000000000L)), Math.floorMod(nanoAdjustment, 1000000000L).toInt)
    def ofMillis(millis: Long): Duration = ofSeconds(Math.floorDiv(millis, 1000L), Math.floorMod(millis, 1000L) * 1000000L)
    def ofNanos(nanos: Long): Duration = ofSeconds(0L, nanos)
    def ofMinutes(minutes: Long): Duration = ofSeconds(Math.multiplyExact(minutes, 60L))
    def ofHours(hours: Long): Duration = ofSeconds(Math.multiplyExact(hours, 3600L))
    def between(start: Instant, end: Instant): Duration = ofSeconds(end.getEpochSecond - start.getEpochSecond, (end.getNano - start.getNano).toLong)
    def ofDays(days: Long): Duration = ofSeconds(Math.multiplyExact(days, 86400L))
    def of(amount: Long, unit: java.time.temporal.TemporalUnit): Duration = ZERO.plus(amount, unit)
    // A count of nanoseconds that may pass a `Long`, refused past the seconds a `Duration` holds.
    private[time] def ofNanosExact(total: BigInt): Duration =
      val billion = BigInt(1000000000L)
      val secs = total / billion
      val rem = total % billion
      val (s, n) = if rem < 0 then (secs - 1, rem + billion) else (secs, rem)
      if s < BigInt(Long.MinValue) || s > BigInt(Long.MaxValue) then throw new ArithmeticException("Exceeds capacity of Duration: " + total)
      new Duration(s.toLong, n.toInt)

  // The machine's zone, by the name the system gives it, or a fixed offset.
  abstract class ZoneId:
    def getId: String
    // The offset at an instant, in seconds.
    private[time] def offsetAt(epochSecond: Long): Int
    // The instant, in seconds, of a local date and time.
    private[time] def instantOf(y: Int, m: Int, d: Int, h: Int, mi: Int, s: Int): Long
    def normalized(): ZoneId = this
    override def equals(other: Any): Boolean = other match
      case z: ZoneId => z.getId == getId
      case _ => false
    override def hashCode: Int = getId.hashCode
    override def toString: String = getId

  object ZoneId:
    // The JDK's: the default `TimeZone`'s zone, which `TimeZone.setDefault` moves.
    def systemDefault(): ZoneId = java.util.TimeZone.getDefault().toZoneId()
    // `UTC`, `GMT`, `Z`, an offset, or the machine's own zone by its name.
    def of(zoneId: String): ZoneId =
      if zoneId == null then throw new NullPointerException("zoneId")
      if zoneId == "Z" || zoneId.startsWith("+") || zoneId.startsWith("-") then ZoneOffset.of(zoneId)
      else if zoneId == "UTC" || zoneId == "GMT" || zoneId == "UT" then new FixedZone(zoneId, ZoneOffset.UTC)
      else if zoneId.startsWith("UTC") || zoneId.startsWith("GMT") then new FixedZone(zoneId.substring(0, 3) + ZoneOffset.of(zoneId.substring(3)).getId, ZoneOffset.of(zoneId.substring(3)))
      else if zoneId == SystemZone.zone.getId then SystemZone.zone
      else throw new UnsupportedOperationException("teq interp has the rules of the machine's own zone (" + SystemZone.zone.getId + ") alone, not " + zoneId)

  private[time] object SystemZone:
    val zone: ZoneId = new ZoneId:
      private val name = timeZoneName()
      def getId: String = name
      private[time] def offsetAt(epochSecond: Long): Int = timeOffset(epochSecond)
      private[time] def instantOf(y: Int, m: Int, d: Int, h: Int, mi: Int, s: Int): Long = timeLocalInstant(y, m, d, h, mi, s)

  private[time] final class FixedZone(id: String, offset: ZoneOffset) extends ZoneId:
    def getId: String = id
    private[time] def offsetAt(epochSecond: Long): Int = offset.getTotalSeconds
    private[time] def instantOf(y: Int, m: Int, d: Int, h: Int, mi: Int, s: Int): Long = daysOf(y, m, d) * 86400L + h * 3600L + mi * 60L + s - offset.getTotalSeconds
    override def normalized(): ZoneId = offset

  final class ZoneOffset private (totalSeconds: Int) extends ZoneId, Comparable[ZoneOffset]:
    def getTotalSeconds: Int = totalSeconds
    def getId: String =
      if totalSeconds == 0 then "Z"
      else
        val a = Math.abs(totalSeconds)
        (if totalSeconds < 0 then "-" else "+") + pad(a / 3600, 2) + ":" + pad(a / 60 % 60, 2) + (if a % 60 != 0 then ":" + pad(a % 60, 2) else "")
    private[time] def offsetAt(epochSecond: Long): Int = totalSeconds
    private[time] def instantOf(y: Int, m: Int, d: Int, h: Int, mi: Int, s: Int): Long = daysOf(y, m, d) * 86400L + h * 3600L + mi * 60L + s - totalSeconds
    def compareTo(other: ZoneOffset): Int = other.getTotalSeconds - totalSeconds

  object ZoneOffset:
    val UTC: ZoneOffset = new ZoneOffset(0)
    def ofTotalSeconds(totalSeconds: Int): ZoneOffset =
      if Math.abs(totalSeconds) > 18 * 3600 then throw new DateTimeException("Zone offset not in valid range: -18:00 to +18:00")
      if totalSeconds == 0 then UTC else new ZoneOffset(totalSeconds)
    def ofHours(hours: Int): ZoneOffset = ofTotalSeconds(hours * 3600)
    def ofHoursMinutes(hours: Int, minutes: Int): ZoneOffset = ofTotalSeconds(hours * 3600 + minutes * 60)
    // `Z`, `+h`, `+hh`, `+hh:mm`, `+hhmm`, `+hh:mm:ss`, `+hhmmss`, with the JDK's messages.
    def of(offsetId: String): ZoneOffset =
      if offsetId == null then throw new NullPointerException("offsetId")
      if offsetId == "Z" then UTC
      else
        var id = offsetId
        def number(at: Int, preceded: Boolean): Int =
          if preceded && id.charAt(at - 1) != ':' then throw new DateTimeException("Invalid ID for ZoneOffset, colon not found when expected: " + id)
          val a = id.charAt(at)
          val b = id.charAt(at + 1)
          if a < '0' || a > '9' || b < '0' || b > '9' then throw new DateTimeException("Invalid ID for ZoneOffset, non numeric characters found: " + id)
          (a - '0') * 10 + (b - '0')
        val (h, m, s) = id.length match
          case 2 | 3 =>
            if id.length == 2 then id = id.substring(0, 1) + "0" + id.substring(1)
            (number(1, false), 0, 0)
          case 5 => (number(1, false), number(3, false), 0)
          case 6 => (number(1, false), number(4, true), 0)
          case 7 => (number(1, false), number(3, false), number(5, false))
          case 9 => (number(1, false), number(4, true), number(7, true))
          case _ => throw new DateTimeException("Invalid ID for ZoneOffset, invalid format: " + id)
        val first = id.charAt(0)
        if first != '+' && first != '-' then throw new DateTimeException("Invalid ID for ZoneOffset, plus/minus not found when expected: " + id)
        val total = h * 3600 + m * 60 + s
        ofTotalSeconds(if first == '-' then -total else total)

  final class LocalDate private[time] (year: Int, month: Int, day: Int) extends Comparable[LocalDate], java.time.temporal.Temporal:
    def getYear: Int = year
    def getMonthValue: Int = month
    def getDayOfMonth: Int = day
    def toEpochDay: Long = daysOf(year, month, day)
    def getMonth: Month = Month.of(month)
    def getDayOfWeek: DayOfWeek = DayOfWeek.of(Math.floorMod(toEpochDay + 3, 7L).toInt + 1)
    def getDayOfYear: Int = (toEpochDay - daysOf(year, 1, 1)).toInt + 1
    def isLeapYear: Boolean = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    def lengthOfMonth: Int = MonthNames.lengthOf(month, isLeapYear)
    def lengthOfYear: Int = if isLeapYear then 366 else 365
    def plusDays(days: Long): LocalDate = LocalDate.ofEpochDay(Math.addExact(toEpochDay, days))
    def minusDays(days: Long): LocalDate = plusDays(-days)
    def plusWeeks(weeks: Long): LocalDate = plusDays(Math.multiplyExact(weeks, 7L))
    def minusWeeks(weeks: Long): LocalDate = plusWeeks(-weeks)
    // The JDK's `plusMonths`: the day of the month kept, or the month's last where it is shorter.
    def plusMonths(months: Long): LocalDate =
      if months == 0L then this
      else
        val count = year * 12L + (month - 1) + months
        val y = Math.floorDiv(count, 12L).toInt
        val m = Math.floorMod(count, 12L).toInt + 1
        new LocalDate(y, m, Math.min(day, MonthNames.lengthOf(m, (y % 4 == 0 && y % 100 != 0) || y % 400 == 0)))
    def minusMonths(months: Long): LocalDate = plusMonths(-months)
    def plusYears(years: Long): LocalDate = plusMonths(Math.multiplyExact(years, 12L))
    def minusYears(years: Long): LocalDate = plusYears(-years)
    def withDayOfMonth(dayOfMonth: Int): LocalDate = LocalDate.of(year, month, dayOfMonth)
    def atStartOfDay(): LocalDateTime = LocalDateTime.of(year, month, day, 0, 0)
    def atStartOfDay(zone: ZoneId): ZonedDateTime = ZonedDateTime.ofLocal(atStartOfDay(), zone)
    def atTime(time: LocalTime): LocalDateTime = LocalDateTime.of(this, time)
    // The JDK's `until`: whole days, weeks, months (by the day of the month) and their multiples.
    def until(endExclusive: java.time.temporal.Temporal, unit: java.time.temporal.TemporalUnit): Long =
      import java.time.temporal.ChronoUnit as U
      val end = LocalDate.from(endExclusive)
      def monthsUntil: Long =
        val start = (year * 12L + month - 1) * 32L + day
        val stop = (end.getYear * 12L + end.getMonthValue - 1) * 32L + end.getDayOfMonth
        (stop - start) / 32L
      if unit == U.DAYS then end.toEpochDay - toEpochDay
      else if unit == U.WEEKS then (end.toEpochDay - toEpochDay) / 7L
      else if unit == U.MONTHS then monthsUntil
      else if unit == U.YEARS then monthsUntil / 12L
      else if unit == U.DECADES then monthsUntil / 120L
      else if unit == U.CENTURIES then monthsUntil / 1200L
      else if unit == U.MILLENNIA then monthsUntil / 12000L
      else throw new java.time.temporal.UnsupportedTemporalTypeException("Unsupported unit: " + unit)
    def atTime(hour: Int, minute: Int): LocalDateTime = LocalDateTime.of(year, month, day, hour, minute)
    def format(formatter: java.time.format.DateTimeFormatter): String = formatter.format(this)
    def compareTo(other: LocalDate): Int = java.lang.Long.compare(toEpochDay, other.toEpochDay)
    def isBefore(other: LocalDate): Boolean = compareTo(other) < 0
    def isAfter(other: LocalDate): Boolean = compareTo(other) > 0
    private[time] def fieldsOrNull: Fields = new Fields(true, year, month, day, false, 0, 0, 0, false, 0, null, null)
    override def equals(other: Any): Boolean = other match
      case d: LocalDate => compareTo(d) == 0
      case _ => false
    override def hashCode: Int = (year & 0xfffff800) ^ ((year << 11) + (month << 6) + day)
    override def toString: String = (if year > 9999 then "+" else "") + pad(year, 4) + "-" + pad(month, 2) + "-" + pad(day, 2)

  object LocalDate:
    def now(): LocalDate = LocalDateTime.now().toLocalDate
    def of(year: Int, month: Int, dayOfMonth: Int): LocalDate =
      checkDate(year, month, dayOfMonth)
      new LocalDate(year, month, dayOfMonth)
    def of(year: Int, month: Month, dayOfMonth: Int): LocalDate = of(year, month.getValue, dayOfMonth)
    def parse(text: CharSequence): LocalDate = parse(text, java.time.format.DateTimeFormatter.ISO_LOCAL_DATE)
    def parse(text: CharSequence, formatter: java.time.format.DateTimeFormatter): LocalDate = formatter.parseResolved(text, 0).asInstanceOf[LocalDate]
    def from(temporal: TemporalAccessor): LocalDate = temporal match
      case d: LocalDate => d
      case d: LocalDateTime => d.toLocalDate
      case z: ZonedDateTime => z.toLocalDate
      case o: OffsetDateTime => o.toLocalDate
      case other => throw new DateTimeException("Unable to obtain LocalDate from TemporalAccessor: " + other + " of type " + other.getClass.getName)
    def ofEpochDay(epochDay: Long): LocalDate =
      val (y, m, d) = dateOf(epochDay)
      new LocalDate(y, m, d)

  final class LocalDateTime private[time] (date: LocalDate, hour: Int, minute: Int, second: Int, nano: Int) extends Comparable[LocalDateTime], java.time.temporal.Temporal:
    def getYear: Int = date.getYear
    def getMonthValue: Int = date.getMonthValue
    def getDayOfMonth: Int = date.getDayOfMonth
    def getHour: Int = hour
    def getMinute: Int = minute
    def getSecond: Int = second
    def getNano: Int = nano
    def toLocalDate: LocalDate = date
    def toLocalTime: LocalTime = LocalTime.of(hour, minute, second, nano)
    def getMonth: Month = date.getMonth
    def getDayOfWeek: DayOfWeek = date.getDayOfWeek
    def atZone(zone: ZoneId): ZonedDateTime = ZonedDateTime.ofLocal(this, zone)
    def atOffset(offset: ZoneOffset): OffsetDateTime = OffsetDateTime.of(this, offset)
    // The JDK's `until`: a date-based unit over the dates, one day less where the time of the end
    // is earlier; a time-based one over the whole difference.
    def until(endExclusive: java.time.temporal.Temporal, unit: java.time.temporal.TemporalUnit): Long =
      val end = LocalDateTime.from(endExclusive)
      if unit.isDateBased() then
        var endDate = end.toLocalDate
        val endTime = end.toLocalTime
        val time = toLocalTime
        if endDate.isAfter(date) && endTime.isBefore(time) then endDate = endDate.minusDays(1L)
        else if endDate.isBefore(date) && endTime.isAfter(time) then endDate = endDate.plusDays(1L)
        date.until(endDate, unit)
      else toInstantAtUtc.until(end.toInstantAtUtc, unit)
    private[time] def toInstantAtUtc: Instant = Instant.ofEpochSecond(toEpochSecond(ZoneOffset.UTC), nano.toLong)
    def toEpochSecond(offset: ZoneOffset): Long = date.toEpochDay * 86400L + hour * 3600L + minute * 60L + second - offset.getTotalSeconds
    def plusSeconds(s: Long): LocalDateTime = LocalDateTime.ofEpochSecond(toEpochSecond(ZoneOffset.UTC) + s, nano, ZoneOffset.UTC)
    def plusMinutes(m: Long): LocalDateTime = plusSeconds(m * 60)
    def plusHours(h: Long): LocalDateTime = plusSeconds(h * 3600)
    def plusDays(d: Long): LocalDateTime = plusSeconds(d * 86400)
    def format(formatter: java.time.format.DateTimeFormatter): String = formatter.format(this)
    def compareTo(other: LocalDateTime): Int =
      val c = java.lang.Long.compare(toEpochSecond(ZoneOffset.UTC), other.toEpochSecond(ZoneOffset.UTC))
      if c != 0 then c else nano - other.getNano
    def isBefore(other: LocalDateTime): Boolean = compareTo(other) < 0
    def isAfter(other: LocalDateTime): Boolean = compareTo(other) > 0
    private[time] def fieldsOrNull: Fields = new Fields(true, date.getYear, date.getMonthValue, date.getDayOfMonth, true, hour, minute, second, true, nano, null, null)
    override def equals(other: Any): Boolean = other match
      case d: LocalDateTime => compareTo(d) == 0
      case _ => false
    override def hashCode: Int = date.hashCode ^ (hour * 3600 + minute * 60 + second + nano)
    override def toString: String =
      date.toString + "T" + pad(hour, 2) + ":" + pad(minute, 2) + (if second == 0 && nano == 0 then "" else ":" + pad(second, 2) + fraction(nano))

  object LocalDateTime:
    def now(): LocalDateTime = ZonedDateTime.now().toLocalDateTime
    def of(year: Int, month: Int, dayOfMonth: Int, hour: Int, minute: Int): LocalDateTime = of(year, month, dayOfMonth, hour, minute, 0, 0)
    def of(year: Int, month: Int, dayOfMonth: Int, hour: Int, minute: Int, second: Int): LocalDateTime = of(year, month, dayOfMonth, hour, minute, second, 0)
    def of(year: Int, month: Int, dayOfMonth: Int, hour: Int, minute: Int, second: Int, nanoOfSecond: Int): LocalDateTime =
      checkTime(hour, minute, second, nanoOfSecond)
      new LocalDateTime(LocalDate.of(year, month, dayOfMonth), hour, minute, second, nanoOfSecond)
    def ofEpochSecond(epochSecond: Long, nanoOfSecond: Int, offset: ZoneOffset): LocalDateTime =
      val local = epochSecond + offset.getTotalSeconds
      val rest = Math.floorMod(local, 86400L).toInt
      new LocalDateTime(LocalDate.ofEpochDay(Math.floorDiv(local, 86400L)), rest / 3600, rest / 60 % 60, rest % 60, nanoOfSecond)
    def ofInstant(instant: Instant, zone: ZoneId): LocalDateTime = ZonedDateTime.ofInstant(instant, zone).toLocalDateTime
    def of(date: LocalDate, time: LocalTime): LocalDateTime = new LocalDateTime(date, time.getHour, time.getMinute, time.getSecond, time.getNano)
    def parse(text: CharSequence): LocalDateTime = parse(text, java.time.format.DateTimeFormatter.ISO_LOCAL_DATE_TIME)
    def parse(text: CharSequence, formatter: java.time.format.DateTimeFormatter): LocalDateTime = formatter.parseResolved(text, 2).asInstanceOf[LocalDateTime]
    def from(temporal: TemporalAccessor): LocalDateTime = temporal match
      case d: LocalDateTime => d
      case z: ZonedDateTime => z.toLocalDateTime
      case o: OffsetDateTime => o.toLocalDateTime
      case other => throw new DateTimeException("Unable to obtain LocalDateTime from TemporalAccessor: " + other + " of type " + other.getClass.getName)

  final class ZonedDateTime private (local: LocalDateTime, offset: ZoneOffset, zone: ZoneId) extends Comparable[ZonedDateTime], java.time.temporal.Temporal:
    def getYear: Int = local.getYear
    def getMonthValue: Int = local.getMonthValue
    def getDayOfMonth: Int = local.getDayOfMonth
    def getHour: Int = local.getHour
    def getMinute: Int = local.getMinute
    def getSecond: Int = local.getSecond
    def getNano: Int = local.getNano
    def getOffset: ZoneOffset = offset
    def getZone: ZoneId = zone
    def toLocalDateTime: LocalDateTime = local
    def toLocalDate: LocalDate = local.toLocalDate
    def toEpochSecond: Long = local.toEpochSecond(offset)
    def toInstant: Instant = Instant.ofEpochSecond(toEpochSecond, local.getNano.toLong)
    def withZoneSameInstant(zone: ZoneId): ZonedDateTime = ZonedDateTime.ofInstant(toInstant, zone)
    def toOffsetDateTime: OffsetDateTime = OffsetDateTime.of(local, offset)
    def toLocalTime: LocalTime = local.toLocalTime
    def getMonth: Month = local.getMonth
    def getDayOfWeek: DayOfWeek = local.getDayOfWeek
    // The JDK's `until`: the end in this zone; a date-based unit over the local date-times, a
    // time-based one over the instants.
    def until(endExclusive: java.time.temporal.Temporal, unit: java.time.temporal.TemporalUnit): Long =
      val end = endExclusive match
        case z: ZonedDateTime => z.withZoneSameInstant(zone)
        case other => ZonedDateTime.ofInstant(Instant.from(other), zone)
      if unit.isDateBased() then local.until(end.toLocalDateTime, unit) else toInstant.until(end.toInstant, unit)
    def plusSeconds(s: Long): ZonedDateTime = ZonedDateTime.ofInstant(toInstant.plusSeconds(s), zone)
    def plusMinutes(m: Long): ZonedDateTime = plusSeconds(m * 60)
    def plusHours(h: Long): ZonedDateTime = plusSeconds(h * 3600)
    def plusDays(d: Long): ZonedDateTime = ZonedDateTime.ofLocal(local.plusDays(d), zone)
    def format(formatter: java.time.format.DateTimeFormatter): String = formatter.format(this)
    def compareTo(other: ZonedDateTime): Int = toInstant.compareTo(other.toInstant)
    def isBefore(other: ZonedDateTime): Boolean = compareTo(other) < 0
    def isAfter(other: ZonedDateTime): Boolean = compareTo(other) > 0
    private[time] def fieldsOrNull: Fields =
      new Fields(true, local.getYear, local.getMonthValue, local.getDayOfMonth, true, local.getHour, local.getMinute, local.getSecond, true, local.getNano, offset, zone)
    override def equals(other: Any): Boolean = other match
      case z: ZonedDateTime => z.toLocalDateTime == local && z.getOffset == offset && z.getZone == zone
      case _ => false
    override def hashCode: Int = local.hashCode ^ offset.hashCode ^ (zone.hashCode * 31)
    override def toString: String =
      local.toString + offset.getId + (if zone.isInstanceOf[ZoneOffset] then "" else "[" + zone.getId + "]")

  final class LocalTime private[time] (hour: Int, minute: Int, second: Int, nano: Int) extends Comparable[LocalTime], java.time.temporal.Temporal:
    def getHour: Int = hour
    def getMinute: Int = minute
    def getSecond: Int = second
    def getNano: Int = nano
    def toSecondOfDay: Int = hour * 3600 + minute * 60 + second
    def toNanoOfDay: Long = toSecondOfDay * 1000000000L + nano
    def plusNanos(n: Long): LocalTime = if n == 0L then this else LocalTime.ofNanoOfDay(Math.floorMod(toNanoOfDay + n % 86400000000000L, 86400000000000L))
    def plusSeconds(s: Long): LocalTime = plusNanos((s % 86400L) * 1000000000L)
    def plusMinutes(m: Long): LocalTime = plusSeconds((m % 1440L) * 60L)
    def plusHours(h: Long): LocalTime = plusSeconds((h % 24L) * 3600L)
    def minusNanos(n: Long): LocalTime = plusNanos(-(n % 86400000000000L))
    def minusSeconds(s: Long): LocalTime = plusSeconds(-(s % 86400L))
    def minusMinutes(m: Long): LocalTime = plusMinutes(-(m % 1440L))
    def minusHours(h: Long): LocalTime = plusHours(-(h % 24L))
    def atDate(date: LocalDate): LocalDateTime = LocalDateTime.of(date, this)
    def format(formatter: java.time.format.DateTimeFormatter): String = formatter.format(this)
    def compareTo(other: LocalTime): Int = java.lang.Long.compare(toNanoOfDay, other.toNanoOfDay)
    def isBefore(other: LocalTime): Boolean = compareTo(other) < 0
    def isAfter(other: LocalTime): Boolean = compareTo(other) > 0
    def until(endExclusive: java.time.temporal.Temporal, unit: java.time.temporal.TemporalUnit): Long =
      import java.time.temporal.ChronoUnit as U
      val nanos = LocalTime.from(endExclusive).toNanoOfDay - toNanoOfDay
      if unit == U.NANOS then nanos
      else if unit == U.MICROS then nanos / 1000L
      else if unit == U.MILLIS then nanos / 1000000L
      else if unit == U.SECONDS then nanos / 1000000000L
      else if unit == U.MINUTES then nanos / 60000000000L
      else if unit == U.HOURS then nanos / 3600000000000L
      else if unit == U.HALF_DAYS then nanos / 43200000000000L
      else throw new java.time.temporal.UnsupportedTemporalTypeException("Unsupported unit: " + unit)
    private[time] def fieldsOrNull: Fields = new Fields(false, 0, 0, 0, true, hour, minute, second, true, nano, null, null)
    override def equals(other: Any): Boolean = other match
      case t: LocalTime => compareTo(t) == 0
      case _ => false
    override def hashCode: Int = java.lang.Long.hashCode(toNanoOfDay)
    override def toString: String =
      pad(hour, 2) + ":" + pad(minute, 2) + (if second == 0 && nano == 0 then "" else ":" + pad(second, 2) + fraction(nano))

  object LocalTime:
    val MIN: LocalTime = new LocalTime(0, 0, 0, 0)
    val MAX: LocalTime = new LocalTime(23, 59, 59, 999999999)
    val MIDNIGHT: LocalTime = MIN
    val NOON: LocalTime = new LocalTime(12, 0, 0, 0)
    def now(): LocalTime = LocalDateTime.now().toLocalTime
    def of(hour: Int, minute: Int): LocalTime = of(hour, minute, 0, 0)
    def of(hour: Int, minute: Int, second: Int): LocalTime = of(hour, minute, second, 0)
    def of(hour: Int, minute: Int, second: Int, nanoOfSecond: Int): LocalTime =
      checkTime(hour, minute, second, nanoOfSecond)
      new LocalTime(hour, minute, second, nanoOfSecond)
    def ofSecondOfDay(secondOfDay: Long): LocalTime =
      if secondOfDay < 0L || secondOfDay > 86399L then throw new DateTimeException("Invalid value for SecondOfDay (valid values 0 - 86399): " + secondOfDay)
      new LocalTime((secondOfDay / 3600L).toInt, (secondOfDay / 60L % 60L).toInt, (secondOfDay % 60L).toInt, 0)
    def ofNanoOfDay(nanoOfDay: Long): LocalTime =
      if nanoOfDay < 0L || nanoOfDay > 86399999999999L then throw new DateTimeException("Invalid value for NanoOfDay (valid values 0 - 86399999999999): " + nanoOfDay)
      val s = nanoOfDay / 1000000000L
      new LocalTime((s / 3600L).toInt, (s / 60L % 60L).toInt, (s % 60L).toInt, (nanoOfDay % 1000000000L).toInt)
    def parse(text: CharSequence): LocalTime = parse(text, java.time.format.DateTimeFormatter.ISO_LOCAL_TIME)
    def parse(text: CharSequence, formatter: java.time.format.DateTimeFormatter): LocalTime = formatter.parseResolved(text, 1).asInstanceOf[LocalTime]
    def from(temporal: TemporalAccessor): LocalTime = temporal match
      case t: LocalTime => t
      case d: LocalDateTime => d.toLocalTime
      case z: ZonedDateTime => z.toLocalTime
      case o: OffsetDateTime => o.toLocalTime
      case other => throw new DateTimeException("Unable to obtain LocalTime from TemporalAccessor: " + other + " of type " + other.getClass.getName)

  // A date-time at a fixed offset from UTC.
  final class OffsetDateTime private (local: LocalDateTime, offset: ZoneOffset) extends Comparable[OffsetDateTime], java.time.temporal.Temporal:
    def getYear: Int = local.getYear
    def getMonthValue: Int = local.getMonthValue
    def getDayOfMonth: Int = local.getDayOfMonth
    def getHour: Int = local.getHour
    def getMinute: Int = local.getMinute
    def getSecond: Int = local.getSecond
    def getNano: Int = local.getNano
    def getOffset: ZoneOffset = offset
    def toLocalDateTime: LocalDateTime = local
    def toLocalDate: LocalDate = local.toLocalDate
    def toLocalTime: LocalTime = local.toLocalTime
    def toEpochSecond: Long = local.toEpochSecond(offset)
    def toInstant: Instant = Instant.ofEpochSecond(toEpochSecond, local.getNano.toLong)
    def atZoneSameInstant(zone: ZoneId): ZonedDateTime = ZonedDateTime.ofInstant(toInstant, zone)
    def format(formatter: java.time.format.DateTimeFormatter): String = formatter.format(this)
    def until(endExclusive: java.time.temporal.Temporal, unit: java.time.temporal.TemporalUnit): Long =
      val end = OffsetDateTime.ofInstant(Instant.from(endExclusive), offset)
      if unit.isDateBased() then local.until(end.toLocalDateTime, unit) else toInstant.until(end.toInstant, unit)
    def compareTo(other: OffsetDateTime): Int =
      val c = toInstant.compareTo(other.toInstant)
      if c != 0 then c else local.compareTo(other.toLocalDateTime)
    def isBefore(other: OffsetDateTime): Boolean = toInstant.isBefore(other.toInstant)
    def isAfter(other: OffsetDateTime): Boolean = toInstant.isAfter(other.toInstant)
    private[time] def fieldsOrNull: Fields =
      new Fields(true, local.getYear, local.getMonthValue, local.getDayOfMonth, true, local.getHour, local.getMinute, local.getSecond, true, local.getNano, offset, offset)
    override def equals(other: Any): Boolean = other match
      case o: OffsetDateTime => o.toLocalDateTime == local && o.getOffset == offset
      case _ => false
    override def hashCode: Int = local.hashCode ^ offset.hashCode
    override def toString: String = local.toString + offset.getId

  object OffsetDateTime:
    def of(local: LocalDateTime, offset: ZoneOffset): OffsetDateTime = new OffsetDateTime(local, offset)
    def ofInstant(instant: Instant, zone: ZoneId): OffsetDateTime =
      val offset = ZoneOffset.ofTotalSeconds(zone.offsetAt(instant.getEpochSecond))
      new OffsetDateTime(LocalDateTime.ofEpochSecond(instant.getEpochSecond, instant.getNano, offset), offset)
    def now(): OffsetDateTime = ofInstant(Instant.now(), ZoneId.systemDefault())

  // The months of the year, January the first.
  @jvmClass("java/time/Month")
  final class Month private (label: String, position: Int) extends Enum[Month]:
    override def name(): String = label
    override def ordinal(): Int = position
    override def toString: String = label
    override def compareTo(that: Month): Int = position - that.ordinal()
    override def hashCode: Int = label.hashCode
    override def equals(that: Any): Boolean = that.asInstanceOf[AnyRef] eq this
    def getValue(): Int = position + 1
    def plus(months: Long): Month = Month.of(Math.floorMod(position + months % 12L, 12L).toInt + 1)
    def minus(months: Long): Month = plus(-(months % 12L))
    def length(leapYear: Boolean): Int = MonthNames.lengthOf(position + 1, leapYear)
    def minLength(): Int = length(false)
    def maxLength(): Int = length(true)

  @jvmClass("java/time/Month")
  object Month:
    val JANUARY: Month = new Month("JANUARY", 0)
    val FEBRUARY: Month = new Month("FEBRUARY", 1)
    val MARCH: Month = new Month("MARCH", 2)
    val APRIL: Month = new Month("APRIL", 3)
    val MAY: Month = new Month("MAY", 4)
    val JUNE: Month = new Month("JUNE", 5)
    val JULY: Month = new Month("JULY", 6)
    val AUGUST: Month = new Month("AUGUST", 7)
    val SEPTEMBER: Month = new Month("SEPTEMBER", 8)
    val OCTOBER: Month = new Month("OCTOBER", 9)
    val NOVEMBER: Month = new Month("NOVEMBER", 10)
    val DECEMBER: Month = new Month("DECEMBER", 11)
    def values(): Array[Month] = Array(JANUARY, FEBRUARY, MARCH, APRIL, MAY, JUNE, JULY, AUGUST, SEPTEMBER, OCTOBER, NOVEMBER, DECEMBER)
    def of(month: Int): Month =
      if month < 1 || month > 12 then throw new DateTimeException("Invalid value for MonthOfYear: " + month)
      values()(month - 1)
    def valueOf(name: String): Month =
      if name == null then throw new NullPointerException("Name is null")
      values().find(_.name() == name).getOrElse(throw new IllegalArgumentException("No enum constant java.time.Month." + name))

  // The days of the week, Monday the first.
  @jvmClass("java/time/DayOfWeek")
  final class DayOfWeek private (label: String, position: Int) extends Enum[DayOfWeek]:
    override def name(): String = label
    override def ordinal(): Int = position
    override def toString: String = label
    override def compareTo(that: DayOfWeek): Int = position - that.ordinal()
    override def hashCode: Int = label.hashCode
    override def equals(that: Any): Boolean = that.asInstanceOf[AnyRef] eq this
    def getValue(): Int = position + 1
    def plus(days: Long): DayOfWeek = DayOfWeek.of(Math.floorMod(position + days % 7L, 7L).toInt + 1)
    def minus(days: Long): DayOfWeek = plus(-(days % 7L))

  @jvmClass("java/time/DayOfWeek")
  object DayOfWeek:
    val MONDAY: DayOfWeek = new DayOfWeek("MONDAY", 0)
    val TUESDAY: DayOfWeek = new DayOfWeek("TUESDAY", 1)
    val WEDNESDAY: DayOfWeek = new DayOfWeek("WEDNESDAY", 2)
    val THURSDAY: DayOfWeek = new DayOfWeek("THURSDAY", 3)
    val FRIDAY: DayOfWeek = new DayOfWeek("FRIDAY", 4)
    val SATURDAY: DayOfWeek = new DayOfWeek("SATURDAY", 5)
    val SUNDAY: DayOfWeek = new DayOfWeek("SUNDAY", 6)
    def values(): Array[DayOfWeek] = Array(MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY)
    def of(dayOfWeek: Int): DayOfWeek =
      if dayOfWeek < 1 || dayOfWeek > 7 then throw new DateTimeException("Invalid value for DayOfWeek: " + dayOfWeek)
      values()(dayOfWeek - 1)
    def valueOf(name: String): DayOfWeek =
      if name == null then throw new NullPointerException("Name is null")
      values().find(_.name() == name).getOrElse(throw new IllegalArgumentException("No enum constant java.time.DayOfWeek." + name))

  object ZonedDateTime:
    def now(): ZonedDateTime = now(ZoneId.systemDefault())
    def now(zone: ZoneId): ZonedDateTime = ofInstant(Instant.now(), zone)
    def ofInstant(instant: Instant, zone: ZoneId): ZonedDateTime =
      val offset = ZoneOffset.ofTotalSeconds(zone.offsetAt(instant.getEpochSecond))
      new ZonedDateTime(LocalDateTime.ofEpochSecond(instant.getEpochSecond, instant.getNano, offset), offset, zone)
    def of(year: Int, month: Int, dayOfMonth: Int, hour: Int, minute: Int, second: Int, nanoOfSecond: Int, zone: ZoneId): ZonedDateTime =
      ofLocal(LocalDateTime.of(year, month, dayOfMonth, hour, minute, second, nanoOfSecond), zone)
    def of(local: LocalDateTime, zone: ZoneId): ZonedDateTime = ofLocal(local, zone)
    private[time] def ofLocal(local: LocalDateTime, zone: ZoneId): ZonedDateTime =
      val epoch = zone.instantOf(local.getYear, local.getMonthValue, local.getDayOfMonth, local.getHour, local.getMinute, local.getSecond)
      ofInstant(Instant.ofEpochSecond(epoch, local.getNano.toLong), zone)

package java.time.format:

  // A pattern as `DateTimeFormatterBuilder.appendPattern` reads it: each letter's count dispatched
  // as `parseField` dispatches it (a number of a width, a short, full or narrow name, a fraction,
  // an offset of each form), a count it refuses refused; quoted literals; `[` and `]` an optional
  // section, printed only when every field in it is there; `{`, `}` and `#` reserved. The letters
  // yuMLdEHkhamsSnXxZ, names in English; another letter of the JDK's is refused with
  // `UnsupportedOperationException`. A zone (`withZone`) applies to a temporal that is an instant,
  // as `DateTimePrintContext` applies it.
  //
  // A text is parsed by the same items (`DateTimeFormatterBuilder`'s parsers: a number of its widths and
  // sign, two digits from 2000, a name of the month, the day or the half of the day, a fraction, a
  // literal), case sensitively, its fields then resolved into the date, the time or the date-time
  // asked for as `Parsed.resolve` resolves them: the ISO formatters strictly, a pattern's smartly (a
  // day past its month's end the month's last day; the hour of a clock with its half of the day).
  final class DateTimeFormatter private (pattern: String, items: List[DateTimeFormatter.Item], zone: java.time.ZoneId, strict: Boolean):
    def withZone(zone: java.time.ZoneId): DateTimeFormatter = new DateTimeFormatter(pattern, items, zone, strict)
    def getZone: java.time.ZoneId = zone
    def format(temporal: java.time.TemporalAccessor): String =
      if temporal == null then throw new NullPointerException("temporal")
      if this eq DateTimeFormatter.ISO_INSTANT then
        return (temporal match
          case i: java.time.Instant => i
          case z: java.time.ZonedDateTime => z.toInstant
          case o: java.time.OffsetDateTime => o.toInstant
          case _ => throw new java.time.temporal.UnsupportedTemporalTypeException("Unsupported field: InstantSeconds")
        ).toString
      val f = (temporal match
        case i: java.time.Instant if zone != null => java.time.ZonedDateTime.ofInstant(i, zone)
        case z: java.time.ZonedDateTime if zone != null && zone != z.getZone => java.time.ZonedDateTime.ofInstant(z.toInstant, zone)
        case other => other
      ).fieldsOrNull
      val sb = new java.lang.StringBuilder()
      items.foreach(_.print(f, sb, false))
      sb.toString
    // The fields of `text`, every one read and nothing left, or the JDK's `DateTimeParseException`.
    private def fieldsOf(text: CharSequence): DateTimeFormatter.Parsed =
      if text == null then throw new NullPointerException("text")
      val p = new DateTimeFormatter.Parsed(text.toString)
      val all = items.forall(_.parse(p))
      val shown = DateTimeFormatter.shown(p.text)
      if !all then throw new DateTimeParseException("Text '" + shown + "' could not be parsed at index " + p.error, p.text, p.error)
      if p.pos < p.text.length then
        throw new DateTimeParseException("Text '" + shown + "' could not be parsed, unparsed text found at index " + p.pos, p.text, p.pos)
      p
    // `text` parsed and resolved into a `LocalDate` (0), a `LocalTime` (1) or a `LocalDateTime` (2).
    private[time] def parseResolved(text: CharSequence, kind: Int): AnyRef =
      val p = fieldsOf(text)
      try p.resolve(kind, strict)
      catch
        case e: java.time.DateTimeException if !e.isInstanceOf[DateTimeParseException] =>
          val ex = new DateTimeParseException("Text '" + DateTimeFormatter.shown(p.text) + "' could not be parsed: " + e.getMessage, p.text, 0)
          ex.initCause(e)
          throw ex
    // `ISO_INSTANT`'s parse: a date, `T`, a time, then `Z` or an offset.
    private[time] def parseInstant(text: CharSequence): java.time.Instant =
      val p = DateTimeFormatter.ISO_OFFSET_DATE_TIME_PARSER.fieldsOf(text)
      try
        val local = p.resolve(2, true).asInstanceOf[java.time.LocalDateTime]
        val offset = p.values.getOrElse('X', 0L).toInt
        java.time.Instant.ofEpochSecond(local.toEpochSecond(java.time.ZoneOffset.UTC) - offset, local.getNano.toLong)
      catch
        case e: java.time.DateTimeException if !e.isInstanceOf[DateTimeParseException] =>
          val ex = new DateTimeParseException("Text '" + DateTimeFormatter.shown(p.text) + "' could not be parsed: " + e.getMessage, p.text, 0)
          ex.initCause(e)
          throw ex
    override def toString: String = pattern

  object DateTimeFormatter:
    // A printer: false, what it wrote taken back, where a field it needs is not there in an
    // optional section; outside one, `UnsupportedTemporalTypeException`.
    private[format] abstract class Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean
      // Reads the item's text at `p.pos` into `p`'s fields; false where it is not there, `p.error`
      // where it failed.
      def parse(p: Parsed): Boolean

    // A parse under way: the text, where it stands, the fields read by their letters (the year of
    // the era `y`, the year `u`, `M`, `d`, the day of the week `E`, the hour `H`, the hour of the
    // clock `h`, the half of the day `a`, `m`, `s`, the nano of the second `n`, the offset `X`).
    private[time] final class Parsed(val text: String):
      var pos = 0
      var error = 0
      val values = scala.collection.mutable.Map.empty[Char, Long]
      def fail(at: Int): Boolean =
        error = at
        false
      // The fields as a `LocalDate`, a `LocalTime` or a `LocalDateTime` (`Parsed.resolveFields`), the
      // year of the era taken as the year where no era is read, a day past its month's end the month's
      // last where the resolution is smart.
      def resolve(kind: Int, strict: Boolean): AnyRef =
        def missing(what: String): Nothing =
          throw new java.time.DateTimeException("Unable to obtain " + what + " from TemporalAccessor: " + values.toSeq.sortBy(_._1).map((k, v) => DateTimeFormatter.fieldName(k) + "=" + v).mkString("{", ", ", "}") + " of type java.time.format.Parsed")
        def date: java.time.LocalDate =
          val year = values.get('u').orElse(values.get('y'))
          (year, values.get('M'), values.get('d')) match
            case (Some(y), Some(m), Some(d0)) =>
              var d = d0
              if !strict && m >= 1 && m <= 12 && d >= 1 && d <= 31 then
                val leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
                d = Math.min(d, java.time.MonthNames.lengthOf(m.toInt, leap).toLong)
              java.time.LocalDate.of(y.toInt, m.toInt, d.toInt)
            case _ => missing("LocalDate")
        def time: java.time.LocalTime =
          val hour = values.get('H').orElse((values.get('h'), values.get('a')) match
            case (Some(h), Some(a)) =>
              if h < 1 || h > 12 then throw new java.time.DateTimeException("Invalid value for ClockHourOfAmPm (valid values 1 - 12): " + h)
              Some(h % 12 + 12 * a)
            case _ => None)
          hour match
            case Some(h) => java.time.LocalTime.of(h.toInt, values.getOrElse('m', 0L).toInt, values.getOrElse('s', 0L).toInt, values.getOrElse('n', 0L).toInt)
            case None => missing("LocalTime")
        kind match
          case 0 => date
          case 1 => time
          case _ => java.time.LocalDateTime.of(date, time)

    // What a text shows in a message: its first 64 characters and `...` past them.
    private[format] def shown(text: String): String = if text.length > 64 then text.substring(0, 64) + "..." else text

    private final class Text(text: String) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean =
        sb.append(text)
        true
      def parse(p: Parsed): Boolean =
        if p.text.startsWith(text, p.pos) then
          p.pos += text.length
          true
        else p.fail(p.pos)

    private final class Optional(items: List[Item]) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean =
        val at = sb.length
        if !items.forall(_.print(f, sb, true)) then sb.setLength(at)
        true
      def parse(p: Parsed): Boolean =
        val (at, before) = (p.pos, p.values.clone())
        if !items.forall(_.parse(p)) then
          p.pos = at
          p.values.clear()
          p.values ++= before
        true

    // The digits from `at`, at most `max` of them: how many.
    private def digits(text: String, at: Int, max: Int): Int =
      var n = 0
      while at + n < text.length && n < max && Character.isDigit(text.charAt(at + n)) do n += 1
      n

    // The field a letter's value is kept under.
    private def key(letter: Char): Char = letter match
      case 'L' => 'M'
      case 'S' => 'n'
      case 'k' => 'K'
      case c => c

    // The value of a letter's field, None where the temporal has no such field.
    private def value(letter: Char, f: java.time.Fields): Option[Long] = letter match
      case 'y' => if f.date then Some((if f.year >= 1 then f.year else 1 - f.year).toLong) else None
      case 'u' => if f.date then Some(f.year.toLong) else None
      case 'M' | 'L' => if f.date then Some(f.month.toLong) else None
      case 'd' => if f.date then Some(f.day.toLong) else None
      case 'E' => if f.date then Some(f.dayOfWeek.toLong) else None
      case 'H' => if f.time then Some(f.hour.toLong) else None
      case 'k' => if f.time then Some((if f.hour == 0 then 24 else f.hour).toLong) else None
      case 'h' => if f.time then Some((if f.hour % 12 == 0 then 12 else f.hour % 12).toLong) else None
      case 'a' => if f.time then Some((if f.hour < 12 then 0 else 1).toLong) else None
      case 'm' => if f.time then Some(f.minute.toLong) else None
      case 's' => if f.time then Some(f.second.toLong) else None
      case 'S' | 'n' => if f.fraction then Some(f.nano.toLong) else None
      case _ => if f.offset != null then Some(f.offset.getTotalSeconds.toLong) else None
    private[format] def fieldName(letter: Char): String = letter match
      case 'y' => "YearOfEra"
      case 'u' => "Year"
      case 'M' | 'L' => "MonthOfYear"
      case 'd' => "DayOfMonth"
      case 'E' => "DayOfWeek"
      case 'H' => "HourOfDay"
      case 'k' => "ClockHourOfDay"
      case 'h' => "ClockHourOfAmPm"
      case 'a' => "AmPmOfDay"
      case 'm' => "MinuteOfHour"
      case 's' => "SecondOfMinute"
      case 'S' | 'n' => "NanoOfSecond"
      case 'K' => "ClockHourOfDay"
      case _ => "OffsetSeconds"
    private def get(letter: Char, f: java.time.Fields, optional: Boolean): Option[Long] =
      val v = value(letter, f)
      if v.isEmpty && !optional then throw new java.time.temporal.UnsupportedTemporalTypeException("Unsupported field: " + fieldName(letter))
      v

    private final val NORMAL = 0
    private final val NOT_NEGATIVE = 1
    private final val EXCEEDS_PAD = 2

    // `NumberPrinterParser`: the digits, a sign by the style, zeros to the least width. Its widths
    // checked when it is made, as `appendValue` checks them for every letter: each from 1 to 19,
    // the maximum at least the minimum; one width when the two are equal and the style
    // NOT_NEGATIVE (`appendValue(field, width)`).
    private final class Number(letter: Char, minWidth: Int, maxWidth: Int, sign: Int) extends Item:
      // The digits the fixed-width numbers right after this one take (`adjacent value parsing`),
      // which a parse of this one leaves them.
      var reserve = 0
      def fixedWidth: Int = if minWidth == maxWidth then minWidth else 0
      if minWidth == maxWidth && sign == NOT_NEGATIVE then
        if minWidth < 1 || minWidth > 19 then throw new IllegalArgumentException("The width must be from 1 to 19 inclusive but was " + minWidth)
      else
        if minWidth < 1 || minWidth > 19 then throw new IllegalArgumentException("The minimum width must be from 1 to 19 inclusive but was " + minWidth)
        if maxWidth < 1 || maxWidth > 19 then throw new IllegalArgumentException("The maximum width must be from 1 to 19 inclusive but was " + maxWidth)
        if maxWidth < minWidth then
          throw new IllegalArgumentException("The maximum width must exceed or equal the minimum width but " + maxWidth + " < " + minWidth)
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get(letter, f, optional) match
        case None => false
        case Some(v) =>
          val digits = Math.abs(v).toString
          if digits.length > maxWidth then
            throw new java.time.DateTimeException("Field " + fieldName(letter) + " cannot be printed as the value " + v + " exceeds the maximum print width of " + maxWidth)
          if v >= 0 then
            if sign == EXCEEDS_PAD && minWidth < 19 && v >= Math.pow(10, minWidth).toLong then sb.append('+')
          else if sign == NOT_NEGATIVE then
            throw new java.time.DateTimeException("Field " + fieldName(letter) + " cannot be printed as the value " + v + " cannot be negative according to the SignStyle")
          else sb.append('-')
          var i = digits.length
          while i < minWidth do
            sb.append('0')
            i += 1
          sb.append(digits)
          true
      // `NumberPrinterParser.parse`: a sign where the style lets one stand, then from the least to
      // the most digits.
      def parse(p: Parsed): Boolean =
        val start = p.pos
        var at = start
        var negative = false
        if at < p.text.length && (p.text.charAt(at) == '+' || p.text.charAt(at) == '-') && sign != NOT_NEGATIVE then
          negative = p.text.charAt(at) == '-'
          at += 1
        val n = Math.min(digits(p.text, at, maxWidth + reserve) - reserve, maxWidth)
        if n < minWidth then p.fail(start)
        else
          val v = p.text.substring(at, at + n).toLong
          p.values(key(letter)) = if negative then -v else v
          p.pos = at + n
          true

    // `ReducedPrinterParser` of two digits from 2000.
    private final class Reduced(letter: Char) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get(letter, f, optional) match
        case None => false
        case Some(v) =>
          sb.append(java.time.pad(Math.abs(v) % 100, 2))
          true
      // Two digits, a year from 2000 to 2099.
      def parse(p: Parsed): Boolean =
        if digits(p.text, p.pos, 2) < 2 then p.fail(p.pos)
        else
          p.values(key(letter)) = 2000L + p.text.substring(p.pos, p.pos + 2).toLong
          p.pos += 2
          true

    // `FractionPrinterParser` of `width` digits, cut rather than rounded.
    private final class Fraction(width: Int) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get('S', f, optional) match
        case None => false
        case Some(v) =>
          sb.append(java.time.pad(v, 9).substring(0, width))
          true
      def parse(p: Parsed): Boolean =
        if digits(p.text, p.pos, width) < width then p.fail(p.pos)
        else
          p.values('n') = (p.text.substring(p.pos, p.pos + width) + "0" * (9 - width)).toLong
          p.pos += width
          true

    // `appendFraction(NANO_OF_SECOND, 0, 9, true)`: the point and the digits as many as there are
    // without the trailing zeros, nothing for none; read back from a point and up to nine digits.
    private final class IsoFraction() extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get('S', f, optional) match
        case None => false
        case Some(v) =>
          if v != 0 then
            var text = java.time.pad(v, 9)
            while text.endsWith("0") do text = text.substring(0, text.length - 1)
            sb.append('.').append(text)
          true
      def parse(p: Parsed): Boolean =
        if p.pos >= p.text.length || p.text.charAt(p.pos) != '.' then true
        else
          val n = digits(p.text, p.pos + 1, 9)
          p.values('n') = (p.text.substring(p.pos + 1, p.pos + 1 + n) + "0" * (9 - n)).toLong
          p.pos += 1 + n
          true

    private val days = Array("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")
    private final val SHORT = 0
    private final val FULL = 1
    private final val NARROW = 2

    // A name: a month's, a day's, AM or PM.
    private final class Name(letter: Char, style: Int) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get(letter, f, optional) match
        case None => false
        case Some(v) =>
          val full = letter match
            case 'M' | 'L' => java.time.MonthNames.names(v.toInt - 1)
            case 'E' => days(v.toInt - 1)
            case _ => if v == 0 then "AM" else "PM"
          sb.append(if letter == 'a' || style == FULL then full else if style == NARROW then full.substring(0, 1) else full.substring(0, 3))
          true
      // The longest of the names that the text starts with, as `TextPrinterParser` matches them.
      def parse(p: Parsed): Boolean =
        val names: Seq[(String, Long)] = letter match
          case 'M' | 'L' => java.time.MonthNames.names.toSeq.zipWithIndex.map((n, i) => (n, (i + 1).toLong))
          case 'E' => days.toSeq.zipWithIndex.map((n, i) => (n, (i + 1).toLong))
          case _ => Seq(("AM", 0L), ("PM", 1L))
        val shown = names.map((n, v) => (if letter == 'a' || style == FULL then n else if style == NARROW then n.substring(0, 1) else n.substring(0, 3), v))
        val found = shown.filter((n, _) => p.text.startsWith(n, p.pos)).sortBy(-_._1.length).headOption
        found match
          case Some((n, v)) =>
            p.values(key(letter)) = v
            p.pos += n.length
            true
          case None => p.fail(p.pos)

    private val offsetPatterns = Array("+HH", "+HHmm", "+HH:mm", "+HHMM", "+HH:MM", "+HHMMss", "+HH:MM:ss", "+HHMMSS", "+HH:MM:SS", "+HHmmss", "+HH:mm:ss")

    // `OffsetIdPrinterParser`: the hours, then the minutes and the seconds as the pattern's style
    // asks (a lowercase part only when it is not zero), the text for no offset.
    private final class Offset(kind: Int, noOffset: String) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get('X', f, optional) match
        case None => false
        case Some(total) =>
          val secs = total.toInt
          if secs == 0 then sb.append(noOffset)
          else
            val style = kind % 11
            val colon = style > 0 && style % 2 == 0
            val hours = Math.abs((secs / 3600) % 100)
            val minutes = Math.abs((secs / 60) % 60)
            val seconds = Math.abs(secs % 60)
            val at = sb.length
            var output = hours
            sb.append(if secs < 0 then "-" else "+")
            sb.append(java.time.pad(hours.toLong, 2))
            if (style >= 3 && style <= 8) || (style >= 9 && seconds > 0) || (style >= 1 && minutes > 0) then
              sb.append(if colon then ":" else "").append(java.time.pad(minutes.toLong, 2))
              output += minutes
              if style == 7 || style == 8 || (style >= 5 && seconds > 0) then
                sb.append(if colon then ":" else "").append(java.time.pad(seconds.toLong, 2))
                output += seconds
            if output == 0 then
              sb.setLength(at)
              sb.append(noOffset)
          true
      // The text for no offset, or a sign and the hours, the minutes and the seconds, a colon
      // between them where the style has one.
      def parse(p: Parsed): Boolean =
        if p.text.startsWith(noOffset, p.pos) then
          p.values('X') = 0L
          p.pos += noOffset.length
          true
        else if p.pos < p.text.length && (p.text.charAt(p.pos) == '+' || p.text.charAt(p.pos) == '-') then
          val negative = p.text.charAt(p.pos) == '-'
          var at = p.pos + 1
          val parts = scala.collection.mutable.ArrayBuffer.empty[Int]
          var going = true
          while going && parts.length < 3 do
            if parts.nonEmpty && at < p.text.length && p.text.charAt(at) == ':' then at += 1
            if digits(p.text, at, 2) == 2 then
              parts += p.text.substring(at, at + 2).toInt
              at += 2
            else going = false
          if parts.isEmpty then p.fail(p.pos)
          else
            val total = parts(0) * 3600 + parts.lift(1).getOrElse(0) * 60 + parts.lift(2).getOrElse(0)
            p.values('X') = (if negative then -total else total).toLong
            p.pos = at
            true
        else p.fail(p.pos)

    // `LocalizedOffsetIdPrinterParser`, full: GMT, then the hours and minutes, the seconds when they
    // are not zero.
    private final class LocalizedOffset() extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get('Z', f, optional) match
        case None => false
        case Some(total) =>
          val secs = total.toInt
          sb.append("GMT")
          if secs != 0 then
            sb.append(if secs < 0 then "-" else "+")
            sb.append(java.time.pad(Math.abs((secs / 3600) % 100).toLong, 2)).append(':').append(java.time.pad(Math.abs((secs / 60) % 60).toLong, 2))
            if secs % 60 != 0 then sb.append(':').append(java.time.pad(Math.abs(secs % 60).toLong, 2))
          true
      def parse(p: Parsed): Boolean = throw new UnsupportedOperationException("teq interp does not parse the localized offset of the pattern letter ZZZZ")

    private def tooMany(c: Char): Nothing = throw new IllegalArgumentException("Too many pattern letters: " + c)

    // `parseField` and the offset letters' cases of `parsePattern`.
    private def field(c: Char, count: Int): Item = c match
      case 'u' | 'y' =>
        if count == 2 then new Reduced(c)
        else new Number(c, count, 19, if count < 4 then NORMAL else EXCEEDS_PAD)
      case 'M' | 'L' | 'E' =>
        count match
          case 1 | 2 => if c == 'E' then new Name(c, SHORT) else if count == 1 then new Number(c, 1, 19, NORMAL) else new Number(c, 2, 2, NOT_NEGATIVE)
          case 3 => new Name(c, SHORT)
          case 4 => new Name(c, FULL)
          case 5 => new Name(c, NARROW)
          case _ => tooMany(c)
      case 'a' => if count == 1 then new Name(c, SHORT) else tooMany(c)
      case 'S' =>
        if count > 9 then throw new IllegalArgumentException("Minimum width must be from 0 to 9 inclusive but was " + count)
        new Fraction(count)
      case 'n' => new Number(c, count, 19, NOT_NEGATIVE)
      case 'd' | 'h' | 'H' | 'k' | 'm' | 's' =>
        if count == 1 then new Number(c, 1, 19, NORMAL) else if count == 2 then new Number(c, 2, 2, NOT_NEGATIVE) else tooMany(c)
      case 'Z' =>
        if count < 4 then new Offset(3, "+0000")
        else if count == 4 then new LocalizedOffset()
        else if count == 5 then new Offset(6, "Z")
        else tooMany(c)
      case 'X' =>
        if count > 5 then tooMany(c)
        new Offset(count + (if count == 1 then 0 else 1), "Z")
      case _ =>
        if count > 5 then tooMany(c)
        new Offset(count + (if count == 1 then 0 else 1), if count == 1 then "+00" else if count % 2 == 0 then "+0000" else "+00:00")

    def ofPattern(pattern: String): DateTimeFormatter =
      if pattern == null then throw new NullPointerException("pattern")
      // The sections open: the outermost first; each a list of its items.
      val sections = scala.collection.mutable.ArrayBuffer(scala.collection.mutable.ListBuffer.empty[Item])
      def add(item: Item): Unit = sections.last += item
      var i = 0
      while i < pattern.length do
        val c = pattern.charAt(i)
        if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') then
          var j = i
          while j < pattern.length && pattern.charAt(j) == c do j += 1
          if !"GuyDMLdQqYwWEecFaKkHhmsSAnNVvzOXxZBgp".contains(c) then throw new IllegalArgumentException("Unknown pattern letter: " + c)
          if !"yuMLdEHkhamsSnXxZ".contains(c) then throw new UnsupportedOperationException("teq interp formats the pattern letters yuMLdEHkhamsSnXxZ alone, not " + c)
          add(field(c, j - i))
          i = j
        else if c == '\'' then
          // A quoted text, `''` a quote inside it, `''` alone a quote.
          var j = i + 1
          var closed = false
          while !closed && j < pattern.length do
            if pattern.charAt(j) == '\'' then
              if j + 1 < pattern.length && pattern.charAt(j + 1) == '\'' then j += 2
              else closed = true
            else j += 1
          if !closed then throw new IllegalArgumentException("Pattern ends with an incomplete string literal: " + pattern)
          val text = pattern.substring(i + 1, j)
          add(new Text(if text.isEmpty then "'" else text.replace("''", "'")))
          i = j + 1
        else if c == '[' then
          sections += scala.collection.mutable.ListBuffer.empty[Item]
          i += 1
        else if c == ']' then
          if sections.length == 1 then throw new IllegalArgumentException("Pattern invalid as it contains ] without previous [")
          val inner = sections.remove(sections.length - 1)
          add(new Optional(inner.toList))
          i += 1
        else if c == '{' || c == '}' || c == '#' then
          throw new IllegalArgumentException("Pattern includes reserved character: '" + c + "'")
        else
          add(new Text(c.toString))
          i += 1
      while sections.length > 1 do
        val inner = sections.remove(sections.length - 1)
        add(new Optional(inner.toList))
      val items = sections.head.toList
      // A number followed straight by fixed-width numbers leaves their digits to them.
      items.tails.foreach {
        case (n: Number) :: rest =>
          val fixed = rest.takeWhile { case m: Number => m.fixedWidth > 0; case _ => false }
          n.reserve = fixed.map(_.asInstanceOf[Number].fixedWidth).sum
        case _ => ()
      }
      new DateTimeFormatter(pattern, items, null, false)

    // The JDK's ISO formatters, resolving strictly: the year of four to ten digits, a sign past
    // four; the seconds and the fraction of a time optional, printed whenever there are seconds.
    private def iso(name: String, items: List[Item]): DateTimeFormatter = new DateTimeFormatter(name, items, null, true)
    private val isoDate = List(new Number('u', 4, 10, EXCEEDS_PAD), new Text("-"), new Number('M', 2, 2, NOT_NEGATIVE), new Text("-"), new Number('d', 2, 2, NOT_NEGATIVE))
    private val isoTime = List(new Number('H', 2, 2, NOT_NEGATIVE), new Text(":"), new Number('m', 2, 2, NOT_NEGATIVE),
      new Optional(List(new Text(":"), new Number('s', 2, 2, NOT_NEGATIVE), new Optional(List(new IsoFraction())))))
    private val isoOffset = new Offset(6, "Z")
    val ISO_LOCAL_DATE: DateTimeFormatter = iso("ParseCaseSensitive(false)(Value(Year,4,10,EXCEEDS_PAD)'-'Value(MonthOfYear,2)'-'Value(DayOfMonth,2))", isoDate)
    val ISO_LOCAL_TIME: DateTimeFormatter = iso("ParseCaseSensitive(false)(Value(HourOfDay,2)':'Value(MinuteOfHour,2)[':'Value(SecondOfMinute,2)[Fraction(NanoOfSecond,0,9,DecimalPoint)]])", isoTime)
    val ISO_LOCAL_DATE_TIME: DateTimeFormatter = iso("ParseCaseSensitive(false)(ISO_LOCAL_DATE'T'ISO_LOCAL_TIME)", isoDate ++ (new Text("T") :: isoTime))
    val ISO_DATE: DateTimeFormatter = iso("ParseCaseSensitive(false)(ISO_LOCAL_DATE[Offset(+HH:MM:ss,'Z')])", isoDate :+ new Optional(List(isoOffset)))
    val ISO_TIME: DateTimeFormatter = iso("ParseCaseSensitive(false)(ISO_LOCAL_TIME[Offset(+HH:MM:ss,'Z')])", isoTime :+ new Optional(List(isoOffset)))
    val ISO_OFFSET_DATE_TIME: DateTimeFormatter = iso("ParseCaseSensitive(false)(ISO_LOCAL_DATE_TIME)Offset(+HH:MM:ss,'Z')", isoDate ++ (new Text("T") :: isoTime) :+ isoOffset)
    // `ISO_INSTANT`: an instant's text as `Instant.toString` writes it (`format`), read back as a UTC
    // date-time with `Z` or an offset (`parseInstant`).
    val ISO_INSTANT: DateTimeFormatter = iso("ParseCaseSensitive(false)InstantSeconds", Nil)
    private[time] val ISO_OFFSET_DATE_TIME_PARSER: DateTimeFormatter = ISO_OFFSET_DATE_TIME
    val BASIC_ISO_DATE: DateTimeFormatter = ofPattern("uuuuMMdd")

  // A text a formatter could not parse: the text and where its parse failed.
  @jvmClass("java/time/format/DateTimeParseException")
  class DateTimeParseException(message: String, parsedData: CharSequence, errorIndex: Int) extends java.time.DateTimeException(message):
    def this(message: String, parsedData: CharSequence, errorIndex: Int, cause: Throwable) =
      this(message, parsedData, errorIndex)
      initCause(cause)
    def getParsedString(): String = parsedData.toString
    def getErrorIndex(): Int = errorIndex

package java.time.temporal:

  // A date, a time or an instant, which a unit's `between` measures from one to the other (`until`).
  @jvmClass("java/time/temporal/Temporal")
  trait Temporal extends java.time.TemporalAccessor:
    def until(endExclusive: Temporal, unit: TemporalUnit): Long

  @jvmClass("java/time/temporal/TemporalUnit")
  trait TemporalUnit:
    def getDuration(): java.time.Duration
    def between(temporal1Inclusive: Temporal, temporal2Exclusive: Temporal): Long
    def isDurationEstimated(): Boolean
    def isDateBased(): Boolean
    def isTimeBased(): Boolean

  // The JDK's units, with their lengths (a month, a year and longer an estimate, a year 365.2425 days).
  @jvmClass("java/time/temporal/ChronoUnit")
  final class ChronoUnit private (label: String, position: Int, length: java.time.Duration) extends Enum[ChronoUnit], TemporalUnit:
    override def name(): String = label
    override def ordinal(): Int = position
    override def compareTo(that: ChronoUnit): Int = position - that.ordinal()
    override def hashCode: Int = label.hashCode
    override def equals(that: Any): Boolean = that.asInstanceOf[AnyRef] eq this
    def getDuration(): java.time.Duration = length
    def between(temporal1Inclusive: Temporal, temporal2Exclusive: Temporal): Long = temporal1Inclusive.until(temporal2Exclusive, this)
    def isDurationEstimated(): Boolean = position >= 7
    def isDateBased(): Boolean = position >= 7 && position != 15
    def isTimeBased(): Boolean = position < 7
    override def toString: String =
      val n = label.toLowerCase
      n.substring(0, 1).toUpperCase + n.substring(1).replace("_d", "D").replace("_y", "Y")

  @jvmClass("java/time/temporal/ChronoUnit")
  object ChronoUnit:
    private def unit(label: String, position: Int, seconds: Long, nanos: Long) =
      new ChronoUnit(label, position, java.time.Duration.ofSeconds(seconds, nanos))
    val NANOS: ChronoUnit = unit("NANOS", 0, 0L, 1L)
    val MICROS: ChronoUnit = unit("MICROS", 1, 0L, 1000L)
    val MILLIS: ChronoUnit = unit("MILLIS", 2, 0L, 1000000L)
    val SECONDS: ChronoUnit = unit("SECONDS", 3, 1L, 0L)
    val MINUTES: ChronoUnit = unit("MINUTES", 4, 60L, 0L)
    val HOURS: ChronoUnit = unit("HOURS", 5, 3600L, 0L)
    val HALF_DAYS: ChronoUnit = unit("HALF_DAYS", 6, 43200L, 0L)
    val DAYS: ChronoUnit = unit("DAYS", 7, 86400L, 0L)
    val WEEKS: ChronoUnit = unit("WEEKS", 8, 7L * 86400L, 0L)
    val MONTHS: ChronoUnit = unit("MONTHS", 9, 31556952L / 12L, 0L)
    val YEARS: ChronoUnit = unit("YEARS", 10, 31556952L, 0L)
    val DECADES: ChronoUnit = unit("DECADES", 11, 31556952L * 10L, 0L)
    val CENTURIES: ChronoUnit = unit("CENTURIES", 12, 31556952L * 100L, 0L)
    val MILLENNIA: ChronoUnit = unit("MILLENNIA", 13, 31556952L * 1000L, 0L)
    val ERAS: ChronoUnit = unit("ERAS", 14, 31556952L * 1000000000L, 0L)
    val FOREVER: ChronoUnit = unit("FOREVER", 15, Long.MaxValue, 999999999L)
    def values(): Array[ChronoUnit] =
      Array(NANOS, MICROS, MILLIS, SECONDS, MINUTES, HOURS, HALF_DAYS, DAYS, WEEKS, MONTHS, YEARS, DECADES, CENTURIES, MILLENNIA, ERAS, FOREVER)
    def valueOf(name: String): ChronoUnit =
      values().find(_.name() == name).getOrElse(throw new IllegalArgumentException("No enum constant java.time.temporal.ChronoUnit." + name))
