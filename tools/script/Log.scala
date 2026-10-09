// What a script says on its way: a line with the time of day in the machine's zone (`date +%T`),
// or in UTC where the old script said `date -u`; and `die`, a message on stderr and status 1.

import java.time.{Instant, ZoneOffset, ZonedDateTime}
import java.time.format.DateTimeFormatter

object Log:
  private val clockFormat = DateTimeFormatter.ofPattern("HH:mm:ss")

  // The time of day now, `HH:mm:ss` in the machine's zone.
  def clock(): String = ZonedDateTime.now().format(clockFormat)

  // The instant now, or the one given in seconds, in UTC with the pattern (`yyyyMMdd'T'HHmmss'Z'`).
  def utc(pattern: String, epochSecond: Long = -1L): String =
    val instant = if epochSecond < 0 then Instant.now() else Instant.ofEpochSecond(epochSecond)
    ZonedDateTime.ofInstant(instant, ZoneOffset.UTC).format(DateTimeFormatter.ofPattern(pattern))

  def stamped(line: String): Unit = println(s"${clock()} $line")

  def die(message: String): Nothing =
    System.err.println(message)
    Script.exit(1)
