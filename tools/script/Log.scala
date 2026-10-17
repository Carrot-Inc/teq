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

  // Why a build tool's run in a log failed, for the line that says it failed: its first error (sbt's and Mill's
  // `[error] ...`, a JVM's `Exception in thread ...`, a launcher's `error: ...`), else the last line it printed (not the
  // statistics /usr/bin/time adds), its colours removed.
  def cause(log: java.nio.file.Path): String =
    val lines = try java.nio.file.Files.readString(log).linesIterator.map(_.replaceAll("\u001b\\[[0-9;]*m", "").trim).toVector catch case _: java.io.IOException => Vector.empty
    def error(l: String) = l.startsWith("[error] ") && !l.startsWith("[error] Total time") && !l.startsWith("[error] \tat ") && l.length > 8 ||
      l.startsWith("Exception in thread") || l.startsWith("error: ") || l.startsWith("ERROR: ")
    val timed = lines.indexWhere(_.startsWith("Command being timed:"))
    val printed = if timed < 0 then lines else lines.take(timed)
    printed.find(error).orElse(printed.reverse.find(_.nonEmpty)).map(_.take(240)).getOrElse(s"nothing in $log")
