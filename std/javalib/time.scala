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
      throw new DateTimeException(if m == 2 && d == 29 then "Invalid date 'February 29' as '" + y + "' is not a leap year" else "Invalid date '" + Month.names(m - 1).toUpperCase + " " + d + "'")
  private[time] def checkTime(h: Int, mi: Int, s: Int, n: Int): Unit =
    if h < 0 || h > 23 then throw new DateTimeException("Invalid value for HourOfDay (valid values 0 - 23): " + h)
    if mi < 0 || mi > 59 then throw new DateTimeException("Invalid value for MinuteOfHour (valid values 0 - 59): " + mi)
    if s < 0 || s > 59 then throw new DateTimeException("Invalid value for SecondOfMinute (valid values 0 - 59): " + s)
    if n < 0 || n > 999999999 then throw new DateTimeException("Invalid value for NanoOfSecond (valid values 0 - 999999999): " + n)

  private[time] object Month:
    val names = Array("January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December")

  // What a formatter reads of a date, a time or an instant.
  trait TemporalAccessor:
    private[time] def fieldsOrNull: Fields

  // A temporal's fields as a formatter reads them: the date's, the time's, the nano of second (an
  // `Instant` has it without a time), the offset; each part there or not.
  private[time] final class Fields(val date: Boolean, val year: Int, val month: Int, val day: Int, val time: Boolean, val hour: Int, val minute: Int, val second: Int, val fraction: Boolean, val nano: Int, val offset: ZoneOffset, val zone: ZoneId):
    def dayOfWeek: Int = Math.floorMod(daysOf(year, month, day) + 3, 7L).toInt + 1

  final class Instant private[time] (seconds: Long, nanos: Int) extends Comparable[Instant], TemporalAccessor:
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
    def plus(d: Duration): Duration = Duration.ofSeconds(seconds + d.getSeconds, nanos.toLong + d.getNano)
    def minus(d: Duration): Duration = Duration.ofSeconds(seconds - d.getSeconds, nanos.toLong - d.getNano)
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
    def systemDefault(): ZoneId = SystemZone.zone
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

  final class LocalDate private[time] (year: Int, month: Int, day: Int) extends Comparable[LocalDate], TemporalAccessor:
    def getYear: Int = year
    def getMonthValue: Int = month
    def getDayOfMonth: Int = day
    def toEpochDay: Long = daysOf(year, month, day)
    def plusDays(days: Long): LocalDate = LocalDate.ofEpochDay(toEpochDay + days)
    def minusDays(days: Long): LocalDate = plusDays(-days)
    def atStartOfDay(): LocalDateTime = LocalDateTime.of(year, month, day, 0, 0)
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
    def ofEpochDay(epochDay: Long): LocalDate =
      val (y, m, d) = dateOf(epochDay)
      new LocalDate(y, m, d)

  final class LocalDateTime private[time] (date: LocalDate, hour: Int, minute: Int, second: Int, nano: Int) extends Comparable[LocalDateTime], TemporalAccessor:
    def getYear: Int = date.getYear
    def getMonthValue: Int = date.getMonthValue
    def getDayOfMonth: Int = date.getDayOfMonth
    def getHour: Int = hour
    def getMinute: Int = minute
    def getSecond: Int = second
    def getNano: Int = nano
    def toLocalDate: LocalDate = date
    def atZone(zone: ZoneId): ZonedDateTime = ZonedDateTime.ofLocal(this, zone)
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

  final class ZonedDateTime private (local: LocalDateTime, offset: ZoneOffset, zone: ZoneId) extends Comparable[ZonedDateTime], TemporalAccessor:
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
  final class DateTimeFormatter private (pattern: String, items: List[DateTimeFormatter.Item], zone: java.time.ZoneId):
    def withZone(zone: java.time.ZoneId): DateTimeFormatter = new DateTimeFormatter(pattern, items, zone)
    def getZone: java.time.ZoneId = zone
    def format(temporal: java.time.TemporalAccessor): String =
      if temporal == null then throw new NullPointerException("temporal")
      val f = (temporal match
        case i: java.time.Instant if zone != null => java.time.ZonedDateTime.ofInstant(i, zone)
        case z: java.time.ZonedDateTime if zone != null && zone != z.getZone => java.time.ZonedDateTime.ofInstant(z.toInstant, zone)
        case other => other
      ).fieldsOrNull
      val sb = new java.lang.StringBuilder()
      items.foreach(_.print(f, sb, false))
      sb.toString
    override def toString: String = pattern

  object DateTimeFormatter:
    // A printer: false, what it wrote taken back, where a field it needs is not there in an
    // optional section; outside one, `UnsupportedTemporalTypeException`.
    private[format] abstract class Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean

    private final class Text(text: String) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean =
        sb.append(text)
        true

    private final class Optional(items: List[Item]) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean =
        val at = sb.length
        if !items.forall(_.print(f, sb, true)) then sb.setLength(at)
        true

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
    private def fieldName(letter: Char): String = letter match
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

    // `ReducedPrinterParser` of two digits from 2000.
    private final class Reduced(letter: Char) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get(letter, f, optional) match
        case None => false
        case Some(v) =>
          sb.append(java.time.pad(Math.abs(v) % 100, 2))
          true

    // `FractionPrinterParser` of `width` digits, cut rather than rounded.
    private final class Fraction(width: Int) extends Item:
      def print(f: java.time.Fields, sb: java.lang.StringBuilder, optional: Boolean): Boolean = get('S', f, optional) match
        case None => false
        case Some(v) =>
          sb.append(java.time.pad(v, 9).substring(0, width))
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
            case 'M' | 'L' => java.time.Month.names(v.toInt - 1)
            case 'E' => days(v.toInt - 1)
            case _ => if v == 0 then "AM" else "PM"
          sb.append(if letter == 'a' || style == FULL then full else if style == NARROW then full.substring(0, 1) else full.substring(0, 3))
          true

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
      new DateTimeFormatter(pattern, sections.head.toList, null)

    val ISO_LOCAL_DATE: DateTimeFormatter = ofPattern("uuuu-MM-dd")
    val BASIC_ISO_DATE: DateTimeFormatter = ofPattern("uuuuMMdd")
