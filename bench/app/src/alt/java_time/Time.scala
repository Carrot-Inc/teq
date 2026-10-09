package meridian.core.time

/** The time types as java.time's, under the names and the surface the model uses (the --java-time
  * flag of bench/app/gen.py; the JVM sides only, Scala.js has no java.time). What java.time spells
  * differently is added as an extension or a companion method. */
type Instant = java.time.Instant
type LocalDate = java.time.LocalDate
type LocalTime = java.time.LocalTime
type Duration = java.time.Duration
type DayOfWeek = java.time.DayOfWeek

private def attempt[A](parse: => A): Option[A] =
  try Some(parse)
  catch case _: java.time.DateTimeException => None

object Instant:
  def ofEpochMilli(ms: Long): Instant = java.time.Instant.ofEpochMilli(ms)
  def ofEpochSecond(s: Long): Instant = java.time.Instant.ofEpochSecond(s)
  def parse(text: String): Instant = java.time.Instant.parse(text)
  def parseOption(text: String): Option[Instant] = attempt(java.time.Instant.parse(text))

object LocalDate:
  def ofEpochDay(day: Long): LocalDate = java.time.LocalDate.ofEpochDay(day)
  def of(year: Int, month: Int, day: Int): LocalDate = java.time.LocalDate.of(year, month, day)
  def parse(text: String): LocalDate = java.time.LocalDate.parse(text)
  def parseOption(text: String): Option[LocalDate] = attempt(java.time.LocalDate.parse(text))
  def isLeap(year: Int): Boolean = java.time.Year.isLeap(year.toLong)
  def lengthOfMonth(year: Int, month: Int): Int = java.time.YearMonth.of(year, month).lengthOfMonth

object LocalTime:
  def ofSecondOfDay(s: Int): LocalTime = java.time.LocalTime.ofSecondOfDay(s.toLong)
  def of(hour: Int, minute: Int): LocalTime = java.time.LocalTime.of(hour, minute)
  def of(hour: Int, minute: Int, second: Int): LocalTime = java.time.LocalTime.of(hour, minute, second)
  def parse(text: String): LocalTime = java.time.LocalTime.parse(text)
  def parseOption(text: String): Option[LocalTime] = attempt(java.time.LocalTime.parse(text))

object Duration:
  val ZERO: Duration = java.time.Duration.ZERO
  def ofMillis(ms: Long): Duration = java.time.Duration.ofMillis(ms)
  def ofSeconds(s: Long): Duration = java.time.Duration.ofSeconds(s)
  def ofMinutes(m: Long): Duration = java.time.Duration.ofMinutes(m)
  def ofHours(h: Long): Duration = java.time.Duration.ofHours(h)
  def between(from: Instant, to: Instant): Duration = java.time.Duration.between(from, to)

object DayOfWeek:
  def values: Array[DayOfWeek] = java.time.DayOfWeek.values()
  def fromOrdinal(i: Int): DayOfWeek = java.time.DayOfWeek.of(i + 1)

extension (instant: Instant)
  def compare(that: Instant): Int = instant.compareTo(that)
  def toLocalDate: LocalDate = instant.atOffset(java.time.ZoneOffset.UTC).toLocalDate
  def toLocalTime: LocalTime = instant.atOffset(java.time.ZoneOffset.UTC).toLocalTime

extension (date: LocalDate)
  def compare(that: LocalDate): Int = date.compareTo(that)
  def until(that: LocalDate): Long = that.toEpochDay - date.toEpochDay
  def atStart: Instant = date.atStartOfDay(java.time.ZoneOffset.UTC).toInstant

extension (time: LocalTime)
  def compare(that: LocalTime): Int = time.compareTo(that)

extension (duration: Duration)
  def compare(that: Duration): Int = duration.compareTo(that)

extension (day: DayOfWeek)
  def isWeekend: Boolean = day == java.time.DayOfWeek.SATURDAY || day == java.time.DayOfWeek.SUNDAY

extension (n: Int)
  def seconds: Duration = java.time.Duration.ofSeconds(n.toLong)
  def minutes: Duration = java.time.Duration.ofMinutes(n.toLong)
  def hours: Duration = java.time.Duration.ofHours(n.toLong)
  def millis: Duration = java.time.Duration.ofMillis(n.toLong)
