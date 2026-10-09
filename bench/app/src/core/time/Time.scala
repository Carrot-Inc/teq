package meridian.core.time

/** Calendar types with the surface of `java.time` that the applications use, over plain
  * numbers: an instant is milliseconds since the epoch, a date a day count, a time of day a
  * count of seconds. ISO text in and out, arithmetic on the numbers. */
final class Instant private (val toEpochMilli: Long) extends Ordered[Instant]:
  def plusMillis(ms: Long): Instant = new Instant(toEpochMilli + ms)
  def plusSeconds(s: Long): Instant = plusMillis(s * 1000L)
  def plus(d: Duration): Instant = plusMillis(d.toMillis)
  def minus(d: Duration): Instant = plusMillis(-d.toMillis)
  def getEpochSecond: Long = Math.floorDiv(toEpochMilli, 1000L)
  def isBefore(that: Instant): Boolean = toEpochMilli < that.toEpochMilli
  def isAfter(that: Instant): Boolean = toEpochMilli > that.toEpochMilli
  def compare(that: Instant): Int = java.lang.Long.compare(toEpochMilli, that.toEpochMilli)
  def toLocalDate: LocalDate = LocalDate.ofEpochDay(Math.floorDiv(toEpochMilli, 86400000L))
  def toLocalTime: LocalTime = LocalTime.ofSecondOfDay((Math.floorMod(toEpochMilli, 86400000L) / 1000L).toInt)
  override def equals(that: Any): Boolean = that match
    case i: Instant => i.toEpochMilli == toEpochMilli
    case _ => false
  override def hashCode: Int = toEpochMilli.hashCode
  override def toString: String =
    val millis = Math.floorMod(toEpochMilli, 1000L)
    val fraction = if millis == 0 then "" else "." + Text.pad3(millis.toInt)
    s"${toLocalDate}T${toLocalTime}$fraction" + "Z"

object Instant:
  val EPOCH: Instant = new Instant(0L)
  def ofEpochMilli(ms: Long): Instant = new Instant(ms)
  def ofEpochSecond(s: Long): Instant = new Instant(s * 1000L)
  def parse(text: String): Instant =
    val t = text.indexOf('T')
    if t < 0 || !text.endsWith("Z") then throw new IllegalArgumentException(s"expected an Instant: $text")
    val date = LocalDate.parse(text.substring(0, t))
    val timePart = text.substring(t + 1, text.length - 1)
    val dot = timePart.indexOf('.')
    val (clock, millis) = if dot < 0 then (timePart, 0L) else (timePart.substring(0, dot), (timePart.substring(dot + 1) + "00").take(3).toLong)
    val time = LocalTime.parse(clock)
    new Instant(date.toEpochDay * 86400000L + time.toSecondOfDay * 1000L + millis)
  def parseOption(text: String): Option[Instant] =
    try Some(parse(text))
    catch case _: IllegalArgumentException => None

final class LocalDate private (val toEpochDay: Long) extends Ordered[LocalDate]:
  private lazy val ymd: (Int, Int, Int) = LocalDate.civil(toEpochDay)
  def getYear: Int = ymd._1
  def getMonthValue: Int = ymd._2
  def getDayOfMonth: Int = ymd._3
  def plusDays(n: Long): LocalDate = new LocalDate(toEpochDay + n)
  def minusDays(n: Long): LocalDate = new LocalDate(toEpochDay - n)
  def plusMonths(n: Int): LocalDate =
    val total = getYear * 12 + (getMonthValue - 1) + n
    val year = Math.floorDiv(total, 12)
    val month = Math.floorMod(total, 12) + 1
    LocalDate.of(year, month, Math.min(getDayOfMonth, LocalDate.lengthOfMonth(year, month)))
  def isBefore(that: LocalDate): Boolean = toEpochDay < that.toEpochDay
  def isAfter(that: LocalDate): Boolean = toEpochDay > that.toEpochDay
  def compare(that: LocalDate): Int = java.lang.Long.compare(toEpochDay, that.toEpochDay)
  def getDayOfWeek: DayOfWeek = DayOfWeek.fromOrdinal(Math.floorMod(toEpochDay + 3, 7L).toInt)
  def atStartOfDay: Instant = Instant.ofEpochMilli(toEpochDay * 86400000L)
  def atTime(time: LocalTime): Instant = Instant.ofEpochMilli(toEpochDay * 86400000L + time.toSecondOfDay * 1000L)
  def until(that: LocalDate): Long = that.toEpochDay - toEpochDay
  override def equals(that: Any): Boolean = that match
    case d: LocalDate => d.toEpochDay == toEpochDay
    case _ => false
  override def hashCode: Int = toEpochDay.hashCode
  override def toString: String = s"${Text.pad4(getYear)}-${Text.pad2(getMonthValue)}-${Text.pad2(getDayOfMonth)}"

object LocalDate:
  def ofEpochDay(day: Long): LocalDate = new LocalDate(day)
  def of(year: Int, month: Int, day: Int): LocalDate = new LocalDate(epochDay(year, month, day))
  def parse(text: String): LocalDate =
    text.split("-").toList match
      case y :: m :: d :: Nil if y.length == 4 && m.length == 2 && d.length == 2 =>
        (y.toIntOption, m.toIntOption, d.toIntOption) match
          case (Some(yy), Some(mm), Some(dd)) if mm >= 1 && mm <= 12 && dd >= 1 && dd <= lengthOfMonth(yy, mm) => of(yy, mm, dd)
          case _ => throw new IllegalArgumentException(s"expected a LocalDate: $text")
      case _ => throw new IllegalArgumentException(s"expected a LocalDate: $text")
  def parseOption(text: String): Option[LocalDate] =
    try Some(parse(text))
    catch case _: IllegalArgumentException => None
  def isLeap(year: Int): Boolean = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
  def lengthOfMonth(year: Int, month: Int): Int = month match
    case 2 => if isLeap(year) then 29 else 28
    case 4 | 6 | 9 | 11 => 30
    case _ => 31
  private def epochDay(year: Int, month: Int, day: Int): Long =
    val y = if month <= 2 then year - 1 else year
    val era = Math.floorDiv(y, 400)
    val yoe = y - era * 400
    val mp = (month + 9) % 12
    val doy = (153 * mp + 2) / 5 + day - 1
    val doe = yoe * 365 + yoe / 4 - yoe / 100 + doy
    era * 146097L + doe - 719468L
  private[time] def civil(epochDay: Long): (Int, Int, Int) =
    val z = epochDay + 719468L
    val era = Math.floorDiv(z, 146097L)
    val doe = (z - era * 146097L).toInt
    val yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365
    val y = yoe + era.toInt * 400
    val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
    val mp = (5 * doy + 2) / 153
    val d = doy - (153 * mp + 2) / 5 + 1
    val m = if mp < 10 then mp + 3 else mp - 9
    (if m <= 2 then y + 1 else y, m, d)

enum DayOfWeek:
  case Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday
  def isWeekend: Boolean = this == Saturday || this == Sunday

final class LocalTime private (val toSecondOfDay: Int) extends Ordered[LocalTime]:
  def getHour: Int = toSecondOfDay / 3600
  def getMinute: Int = toSecondOfDay / 60 % 60
  def getSecond: Int = toSecondOfDay % 60
  def plusMinutes(n: Int): LocalTime = LocalTime.ofSecondOfDay(Math.floorMod(toSecondOfDay + n * 60, 86400))
  def plusHours(n: Int): LocalTime = plusMinutes(n * 60)
  def isBefore(that: LocalTime): Boolean = toSecondOfDay < that.toSecondOfDay
  def isAfter(that: LocalTime): Boolean = toSecondOfDay > that.toSecondOfDay
  def compare(that: LocalTime): Int = Integer.compare(toSecondOfDay, that.toSecondOfDay)
  override def equals(that: Any): Boolean = that match
    case t: LocalTime => t.toSecondOfDay == toSecondOfDay
    case _ => false
  override def hashCode: Int = toSecondOfDay
  override def toString: String =
    val base = s"${Text.pad2(getHour)}:${Text.pad2(getMinute)}"
    if getSecond == 0 then base else s"$base:${Text.pad2(getSecond)}"

object LocalTime:
  val MIDNIGHT: LocalTime = new LocalTime(0)
  val NOON: LocalTime = new LocalTime(43200)
  def ofSecondOfDay(s: Int): LocalTime = new LocalTime(s)
  def of(hour: Int, minute: Int): LocalTime = new LocalTime(hour * 3600 + minute * 60)
  def of(hour: Int, minute: Int, second: Int): LocalTime = new LocalTime(hour * 3600 + minute * 60 + second)
  def parse(text: String): LocalTime =
    text.split(":").toList.map(_.toIntOption) match
      case Some(h) :: Some(m) :: Nil if h < 24 && m < 60 => of(h, m)
      case Some(h) :: Some(m) :: Some(s) :: Nil if h < 24 && m < 60 && s < 60 => of(h, m, s)
      case _ => throw new IllegalArgumentException(s"expected a LocalTime: $text")
  def parseOption(text: String): Option[LocalTime] =
    try Some(parse(text))
    catch case _: IllegalArgumentException => None

final class Duration private (val toMillis: Long) extends Ordered[Duration]:
  def toSeconds: Long = toMillis / 1000L
  def toMinutes: Long = toMillis / 60000L
  def plus(that: Duration): Duration = new Duration(toMillis + that.toMillis)
  def multipliedBy(n: Long): Duration = new Duration(toMillis * n)
  def isZero: Boolean = toMillis == 0L
  def compare(that: Duration): Int = java.lang.Long.compare(toMillis, that.toMillis)
  override def equals(that: Any): Boolean = that match
    case d: Duration => d.toMillis == toMillis
    case _ => false
  override def hashCode: Int = toMillis.hashCode
  override def toString: String =
    val h = toMillis / 3600000L
    val m = toMillis / 60000L % 60
    val s = toMillis / 1000L % 60
    val ms = toMillis % 1000L
    val tail = if ms == 0 then s"${s}S" else s"$s.${Text.pad3(ms.toInt)}S"
    "PT" + (if h > 0 then s"${h}H" else "") + (if m > 0 then s"${m}M" else "") + (if s > 0 || ms > 0 || (h == 0 && m == 0) then tail else "")

object Duration:
  val ZERO: Duration = new Duration(0L)
  def ofMillis(ms: Long): Duration = new Duration(ms)
  def ofSeconds(s: Long): Duration = new Duration(s * 1000L)
  def ofMinutes(m: Long): Duration = new Duration(m * 60000L)
  def ofHours(h: Long): Duration = new Duration(h * 3600000L)
  def between(from: Instant, to: Instant): Duration = new Duration(to.toEpochMilli - from.toEpochMilli)

extension (n: Int)
  def seconds: Duration = Duration.ofSeconds(n.toLong)
  def minutes: Duration = Duration.ofMinutes(n.toLong)
  def hours: Duration = Duration.ofHours(n.toLong)
  def millis: Duration = Duration.ofMillis(n.toLong)

private object Text:
  def pad2(n: Int): String = if n < 10 then "0" + n else n.toString
  def pad3(n: Int): String = if n < 10 then "00" + n else if n < 100 then "0" + n else n.toString
  def pad4(n: Int): String = if n < 0 then "-" + pad4(-n) else if n < 10 then "000" + n else if n < 100 then "00" + n else if n < 1000 then "0" + n else n.toString

// The names java.time spells differently, defined at the top level so that the --java-time
// variant of this file can export them under the same names; here the members take precedence.
extension (instant: Instant)
  def compare(that: Instant): Int = instant.compareTo(that)
  def toLocalDate: LocalDate = instant.toLocalDate
  def toLocalTime: LocalTime = instant.toLocalTime
extension (date: LocalDate)
  def compare(that: LocalDate): Int = date.compareTo(that)
  def until(that: LocalDate): Long = date.until(that)
  def atStart: Instant = date.atStartOfDay
extension (time: LocalTime)
  def compare(that: LocalTime): Int = time.compareTo(that)
extension (duration: Duration)
  def compare(that: Duration): Int = duration.compareTo(that)
extension (day: DayOfWeek)
  def isWeekend: Boolean = day.isWeekend
